global start
extern long_mode_start


section .text
bits 32

start:
	mov esp, stack_top
	mov edi, ebx

	call check_multiboot
	call check_cpuid
	call check_longmode

	call setup_pagetables
	call enable_paging

	lgdt [gdt64.pointer]
	jmp gdt64.code:long_mode_start


	; prints "OK" to screen
	; long_mode_start call overrides this
	;mov dword [0xb8000], 0x2f4b2f4f
	;hlt


check_multiboot:
	cmp eax, 0x36D76289
	jne .no_multiboot
	ret

.no_multiboot:
	mov al, "0"
	jmp error

check_cpuid:
	; checks if CPUID supported by CPU

	pushfd
	pop eax

	mov ecx, eax

	xor eax, 1 << 21 	; flip ID bit

	push eax
	popfd

	pushfd
	pop eax

	push ecx
	popfd

	cmp eax,ecx
	je .no_cpuid
	ret

.no_cpuid:
	mov al, "1"
	jmp error


check_longmode:
	; checks if CPU supports long mode

	mov eax, 0x80000000
	cpuid
	cmp eax, 0x80000001
	jb .no_longmode

	mov eax, 0x80000001
	cpuid
	test edx, 1 << 29
	jz .no_longmode
	ret

.no_longmode:
	mov al, "2"
	jmp error

setup_pagetables:
	mov eax, p4_table	; map P4 table recursively
	or eax, 0b11		; present + writable
	mov [p4_table + 511 * 8], eax

	mov eax, p3_table	; add p4 entry into p3
	or eax, 0b11		; present + writable
	mov [p4_table], eax

	mov eax, p2_table	; add p3 entry into p2
	or eax, 0b11
	mov [p3_table], eax

	; map p2 to a 2MiB table
	mov ecx, 0

.map_p2_table:
	; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
	mov eax, 0x200000 		; 2 MiB
	mul ecx				; start address of ecx-th page
	or eax, 0b10000011 		; present + writable + huge
	mov [p2_table + ecx * 8], eax 	; map ecx-th entry

	inc ecx				; increment counter
	cmp ecx, 512			; if counter == 512, whole p2 table is mapped
	jne .map_p2_table		; else map next entry

	ret

enable_paging:
	; load P4 into cr3 register. CPU uses this to access P4 table
	mov eax, p4_table
	mov cr3, eax

	; enable PAE flag in cr4
	mov eax, cr4
	or eax, 1 << 5
	mov cr4, eax

	; set long mode bit in the EFER MSR (model specific register)
	mov ecx, 0xC0000080
	rdmsr
	or eax, 1 << 8
	wrmsr

	; enable paging in cr0 register
	mov eax, cr0
	or eax, 1 << 31
	mov cr0, eax

	ret

error:
	mov dword [0xb8000], 0x4F524F45
	mov dword [0xb8004], 0x4F3A4F52
	mov dword [0xb8008], 0x4F204F20
	mov byte [0xb800A], al
	hlt


section .bss
align 4096
p4_table:
	resb 4096
p3_table:
	resb 4096
p2_table:
	resb 4096
stack_bottom:
	resb 4096*4
stack_top:


section .rodata
gdt64:
	dq 0 						; zero entry
.code: equ $ - gdt64
	dq (1<<43) | (1<<44) | (1<<47) | (1<<53)	; code segment
.pointer:
	dw $ - gdt64 - 1
	dq gdt64

