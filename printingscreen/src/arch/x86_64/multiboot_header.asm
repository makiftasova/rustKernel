section .multiboot_header
header_start: 
	dd 0xe85250d6			; multiboot 2 magic number
	dd 0				; architecture 0 (i386 protected mode)
	dd header_end - header_start	; header length
	;checksum
	dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))
	
	; optional multiboot tags

	; required end tag
	dw 0				; type
	dw 0				; flags
	dw 8				; size
header_end:
