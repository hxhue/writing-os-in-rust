use crate::{gdt, hlt_loop, print, println};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

// Remap PIC interrupts index range.
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// This implementation uses an IDT at an unfixed address.
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        let double_fault_opts = idt.double_fault.set_handler_fn(double_fault_handler);
        unsafe {
            // x64 TSS contains an interrupt stack table, which will not be
            // used unless we set stack index for exceptions explicitly.
            // After this setting, when a double fault happens, CPU will query
            // ist[DOUBLE_FAULT_IST_INDEX], and use the recorded address as
            // the bottom of interrupt stack. That's why TSS must be loaded
            // before IDT.
            double_fault_opts.set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

// 2022-06-16 15:15:13
// Why is GDT not set before IDT?

// 2022-06-16 16:13:28
// It seems that IDT does not have to work with GDT. IDT can also be allocated
// outside segments registered in GDT. But to intialize a kernel, GDT must be
// taken care of in some way, which happens before IDT.
// Maybe "bootimage" dependency solves this problem for us?

// 2022-06-16 16:33:34
// Some code dumped from the head of the binary:
//     0:  31 c0                   xor    eax,eax
//     2:  8e d8                   mov    ds,eax
//     4:  8e c0                   mov    es,eax
//     6:  8e d0                   mov    ss,eax
//     8:  8e e0                   mov    fs,eax
//     a:  8e e8                   mov    gs,eax
//     c:  fc                      cld
//     d:  bc 00 7c be 12          mov    esp,0x12be7c00
//     12: 7d 66                   jge    0x7a
//     14: e8 a2 00 00 00          call   0xbb
//     19: e4 92                   in     al,0x92
//     1b: a8 02                   test   al,0x2
//     1d: 75 06                   jne    0x25
//     1f: 0c 02                   or     al,0x2
//     21: 24 fe                   and    al,0xfe
//     23: e6 92                   out    0x92,al
//     25: fa                      cli
//     26: 1e                      (bad)
//     27: 06                      (bad)
//     28: 0f 01 16                lgdt   [rsi]
// Last line contains lgdt.

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // print!(".");
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    // `HandleControl::Ignore` means handle ctrl+[a-z] like normal characters, i.e. [a-z].
    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
            Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
        );
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // Well... keyboard processor is a state machine. Because a single byte may
    // not be a complete character.
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

use x86_64::structures::idt::PageFaultErrorCode;

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}
