use SBEntryState::{ALLOCATED, COMMITTED, IDLE, READY};

use crate::cpu::CPUConfig;
use crate::instructions::instructions::DWordType;

enum SBEntryState {
    // not used.
    IDLE,
    // it is allocated for a store
    ALLOCATED,
    // the value is stored, but it is still in speculative state. So there
    // is no guarantee that the store is going to be written to main memory
    READY,
    // the value is stored, and is not any longer in speculative state
    // and is guaranteed to be written to main memory
    COMMITTED,
}

struct SBEntry {
    value: DWordType,
    addr: DWordType,
    state: SBEntryState,
}

impl SBEntry {
    fn reset(&mut self) {
        self.state = IDLE;
        self.addr = 0;
        self.value = 0;
    }
}

pub(crate) struct SB {
    head: u64,
    tail: u64,
    entries: Vec<SBEntry>,
    capacity: u16,
    lfb_count: u8,
}

impl SB {
    pub(crate) fn new(cpu_config: &CPUConfig) -> SB {
        let mut entries = Vec::with_capacity(cpu_config.sb_capacity as usize);
        for _ in 0..cpu_config.sb_capacity {
            entries.push(SBEntry {
                value: 0,
                addr: 0,
                state: IDLE,
            })
        }

        SB {
            capacity: cpu_config.sb_capacity,
            head: 0,
            tail: 0,
            entries,
            lfb_count: cpu_config.lfb_count,
        }
    }

    pub(crate) fn size(&self) -> u16 {
        return (self.tail - self.head) as u16;
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.size() == 0
    }

    pub(crate) fn has_space(&self) -> bool {
        return self.size() < self.capacity;
    }

    pub(crate) fn allocate(&mut self) -> u16 {
        assert!(self.has_space(), "StoreBuffer: can't allocate because there is no space");

        let index = self.to_index(self.tail);
        self.entries[index].state = ALLOCATED;
        self.tail += 1;
        return index as u16;
    }

    fn to_index(&self, seq: u64) -> usize {
        (seq % self.capacity as u64) as usize
    }

    pub(crate) fn store(&mut self, index: u16, addr: DWordType, value: DWordType) {
        let sb_entry = &mut self.entries[index as usize];

        match sb_entry.state {
            ALLOCATED => {
                sb_entry.addr = addr;
                sb_entry.value = value;
                sb_entry.state = READY;
            }
            _ => unreachable!(),
        }
    }

    pub(crate) fn commit(&mut self, index: u16) {
        let sb_entry = &mut self.entries[index as usize];

        match sb_entry.state {
            READY => sb_entry.state = COMMITTED,
            _ => unreachable!(),
        }
    }

    pub(crate) fn flush(&mut self) {
        // to flush, we go backwards from the tail and 'deallocate' every store
        // until a committed store is found. Behind that committed store can only
        // be other committed stores because commits are done in order (retire).

        for k in (self.head..self.tail).rev() {
            let index = self.to_index(k);
            let sb_entry = &mut self.entries[index];
            match sb_entry.state {
                ALLOCATED |
                READY => {
                    sb_entry.reset();
                    self.tail -= 1;
                }
                COMMITTED => {
                    break;
                }
                _ => unreachable!(),
            }
        }
    }

    pub(crate) fn do_cycle(&mut self, memory: &mut Vec<DWordType>) {
        for _ in 0..self.lfb_count {
            if self.is_empty() {
                break;
            }

            let index = self.to_index(self.head);
            let sb_entry = &mut self.entries[index];
            match sb_entry.state {
                ALLOCATED |
                READY => {}
                COMMITTED => {
                    // write the store to memory
                    memory[sb_entry.addr as usize] = sb_entry.value;
                    sb_entry.reset();
                    self.head += 1;
                }
                _ => unreachable!(),
            }
        }
    }
}
