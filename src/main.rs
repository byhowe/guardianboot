#![no_main]
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::char;

use uefi::entry;
use uefi::proto::console::text::Key;
use uefi::table::Boot;
use uefi::table::SystemTable;
use uefi::Handle;
use uefi::Status;
use uefi_services::print;
use uefi_services::println;

#[entry]
fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status
{
    uefi_services::init(&mut system_table).unwrap();
    println!("Hello World!");
    loop {
        let password = read_password(&mut system_table).unwrap();
        println!("The password entered is: {:?}", password);
    }
    Status::SUCCESS
}

/// Read password from stdin.
fn read_password(system_table: &mut SystemTable<Boot>) -> Result<String, String>
{
    print!("[guardianboot] Please enter your password: ");
    let mut password = String::new();

    let cursor_visible = system_table.stdout().cursor_visible();
    system_table.stdout().enable_cursor(true).unwrap();

    loop {
        // Pause until a keyboard event occurs.
        let mut events = unsafe { [system_table.stdin().wait_for_key_event().unsafe_clone()] };
        system_table
            .boot_services()
            .wait_for_event(&mut events)
            .unwrap();
        let key = system_table.stdin().read_key().unwrap();
        match key {
            Some(Key::Printable(ch)) => match char::from(ch) {
                // enter key is pressed. this means the password is enetered.
                '\r' => {
                    println!();
                    break;
                }
                // backspace key. remove the last enetered character.
                '\x08' => {
                    if password.pop().is_some() {
                        // visually remove last character
                        print!("\x08 \x08");
                    }
                }
                ch => {
                    password.push(ch);
                    print!("*");
                }
            },
            _ => {}
        }
    }

    system_table.stdout().enable_cursor(cursor_visible).unwrap();
    Ok(password)
}
