
// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use crate::{print, println};
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use core::{pin::Pin, task::{Poll, Context}};
use futures_util::{stream::{Stream, StreamExt}, task::AtomicWaker};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

// ---------------------------------------------------------------------------
// STATICS
// ---------------------------------------------------------------------------

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

// ---------------------------------------------------------------------------
// DATA STRUCTURES
// ---------------------------------------------------------------------------

/// A stream object connected to the keyboard incoming scancodes
pub struct ScancodeStream {
    _private: ()
}

impl ScancodeStream {

    /// Initialise a new scancode stream.
    /// 
    /// This function must only be called once.
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new must only be called once");
        ScancodeStream { 
            _private: ()
        }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    /// Get the next item in the stream
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        // Get the queue
        let queue = SCANCODE_QUEUE.try_get()
            .expect("[KBD-ERROR] Scancode queue not initialised");

        // If a scancode already is available extract it now rather than going
        // through the expensive waker process
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        // If no scancode then register the waker so the executor can awaken
        // the keyboard when a key is pressed
        WAKER.register(&cx.waker());

        // If there's a scancode in the queue return it, otherwise pending.
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            },
            Err(crossbeam_queue::PopError) => Poll::Pending
        }
        
    }
}

// ---------------------------------------------------------------------------
// PUBLIC FUNCTIONS
// ---------------------------------------------------------------------------

/// Push a new scancode into the queue.
/// 
/// Should be called from the keyboard interrupt handler.
pub(crate) fn push_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("[KBD-ERROR] Scancode push failed, dropping input");
        }
        else {
            // Awaken the background worker task since a new scancode was 
            // pushed.
            WAKER.wake();
        }
    }
    else {
        println!("[KBD-ERROR] Scancode queue not initialised");
    }
}

/// Print the keypresses from the keyboard
pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        layouts::Uk105Key,
        ScancodeSet1,
        HandleControl::Ignore);

    // While there are scancodes available process and print they key
    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(chr) => print!("{}", chr),
                    DecodedKey::RawKey(key) => print!("{:?}", key)
                }
            }
        }
    }
}