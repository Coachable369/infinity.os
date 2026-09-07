//! Freestanding memory operations; no libc or recursive compiler intrinsics.
use core::ptr::{read_volatile, write_volatile};

// ------------------------=
// FUNC: copy
// DESC: Copies a valid nonoverlapping byte range without a host runtime dependency.
// ------------------=
pub unsafe fn copy(destination: *mut u8, source: *const u8, count: usize) {
    let word = core::mem::size_of::<usize>();
    let mut index = 0;
    if (destination as usize ^ source as usize) & (word - 1) == 0 {
        while index < count && (destination.add(index) as usize) & (word - 1) != 0 {
            write_volatile(destination.add(index), read_volatile(source.add(index)));
            index += 1;
        }
        while count - index >= word * 4 {
            for offset in 0..4 {
                let at = index + offset * word;
                write_volatile(
                    destination.add(at).cast::<usize>(),
                    read_volatile(source.add(at).cast::<usize>()),
                );
            }
            index += word * 4;
        }
        while count - index >= word {
            write_volatile(
                destination.add(index).cast::<usize>(),
                read_volatile(source.add(index).cast::<usize>()),
            );
            index += word;
        }
    }
    while index < count {
        write_volatile(destination.add(index), read_volatile(source.add(index)));
        index += 1;
    }
}

// ------------------------=
// FUNC: move_bytes
// DESC: Moves a valid possibly overlapping byte range in the safe direction.
// ------------------=
pub unsafe fn move_bytes(destination: *mut u8, source: *const u8, count: usize) {
    if destination as usize <= source as usize {
        copy(destination, source, count);
    } else {
        let word = core::mem::size_of::<usize>();
        let mut index = count;
        if (destination as usize ^ source as usize) & (word - 1) == 0 {
            while index != 0 && (destination.add(index) as usize) & (word - 1) != 0 {
                index -= 1;
                write_volatile(destination.add(index), read_volatile(source.add(index)));
            }
            while index >= word {
                index -= word;
                write_volatile(
                    destination.add(index).cast::<usize>(),
                    read_volatile(source.add(index).cast::<usize>()),
                );
            }
        }
        while index != 0 {
            index -= 1;
            write_volatile(destination.add(index), read_volatile(source.add(index)));
        }
    }
}

// ------------------------=
// FUNC: fill
// DESC: Fills a valid byte range without calling a compiler-generated memset.
// ------------------=
pub unsafe fn fill(destination: *mut u8, value: u8, count: usize) {
    let word = core::mem::size_of::<usize>();
    let packed = (usize::MAX / 255) * usize::from(value);
    let mut index = 0;
    while index < count && (destination.add(index) as usize) & (word - 1) != 0 {
        write_volatile(destination.add(index), value);
        index += 1;
    }
    while count - index >= word {
        write_volatile(destination.add(index).cast::<usize>(), packed);
        index += word;
    }
    while index < count {
        write_volatile(destination.add(index), value);
        index += 1;
    }
}
