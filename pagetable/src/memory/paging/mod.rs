pub use self::entry::*;
use core::ptr::Unique;
use memory::{PAGE_SIZE, Frame, FrameAllocator};
use self::table::{Table, Level4};

mod entry;
mod table;

const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

pub struct Page {
    number: usize,
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        assert!(
            address < 0x0000_8000_0000_0000 || address >= 0xFFFF_8000_0000_0000,
            "Invalid address: 0x{:x}",
            address
        );
        Page { number: address / PAGE_SIZE }
    }

    fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0x1FF /* get least significant 9 bits */
    }

    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0x1FF /* get least significant 9 bits */
    }

    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0x1FF /* get least significant 9 bits */
    }

    fn p1_index(&self) -> usize {
        (self.number >> 0) & 0x1FF /* get least significant 9 bis */
    }
}

pub struct ActivePageTable {
    p4: Unique<Table<Level4>>,
}

impl ActivePageTable {
    pub unsafe fn new() -> ActivePageTable {
        ActivePageTable { p4: Unique::new(table::P4).unwrap() }
    }

    fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(Page::containing_address(virtual_address))
            .map(|frame| frame.number * PAGE_SIZE + offset)
    }

    fn translate_page(&self, page: Page) -> Option<Frame> {
        use self::entry::HUGE_PAGE;

        let p3 = self.p4().next_table(page.p4_index());

        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];
                // 1GiB page?
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(HUGE_PAGE) {
                        // address must be 1GiB aligned
                        assert!(start_frame.number % (ENTRY_COUNT * ENTRY_COUNT) == 0);
                        return Some(Frame {
                            number: start_frame.number + page.p2_index() * ENTRY_COUNT +
                                page.p1_index(),
                        });
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];
                    // 2MiB page?
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(HUGE_PAGE) {
                            // address must be 2MiB aligned
                            assert!(start_frame.number % ENTRY_COUNT == 0);
                            return Some(Frame { number: start_frame.number + page.p1_index() });
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index()].pointed_frame())
            .or_else(huge_page)
    }

    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let p4 = self.p4_mut();
        let mut p3 = p4.next_table_create(page.p4_index(), allocator);
        let mut p2 = p3.next_table_create(page.p3_index(), allocator);
        let mut p1 = p2.next_table_create(page.p2_index(), allocator);

        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | PRESENT);
    }

    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let frame = allocator.allocate_frame().expect("Out of memory");
        self.map_to(page, frame, flags, allocator)
    }

    pub fn identitiy_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }

    fn unmap<A>(&mut self, page: Page, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        assert!(self.translate(page.start_address()).is_some());

        let p1 = self.p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("Mapping code does not support huge pages");

        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();

        use x86_64::instructions::tlb;
        use x86_64::VirtualAddress;
        tlb::flush(VirtualAddress(page.start_address()));


        // TODO free p1, p2 p3 tables if not empty
        //allocator.deallocate_frame(frame);
    }
}

pub fn test_paging<A>(allocator: &mut A)
where
    A: FrameAllocator,
{
    let mut page_table = unsafe { ActivePageTable::new() };

    let addr = 42 * 512 * 512 * 4096; //42nd P3 entry

    println!("begin allocation addr: 0x{:x}", addr);

    let page = Page::containing_address(addr);
    let frame = allocator.allocate_frame().expect(
        "Can't allocate new frame",
    );

    println!(
        "None = {:?}, map to {:?}",
        page_table.translate(addr),
        frame
    );
    page_table.map_to(page, frame, EntryFlags::empty(), allocator);
    println!("Some = {:?}", page_table.translate(addr));
    println!("next free frame: {:?}", allocator.allocate_frame());

    println!("{:#x}", unsafe {
        *(Page::containing_address(addr).start_address() as *const u64)
    });

    page_table.unmap(Page::containing_address(addr), allocator);
    println!("None = {:?}", page_table.translate(addr));



}
