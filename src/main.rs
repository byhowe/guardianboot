#![no_main]
#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ptr;
use core::slice;

use uefi::cstr16;
use uefi::entry;
use uefi::fs::FileSystem;
use uefi::fs::Path;
use uefi::proto::console::text::Key;
use uefi::proto::device_path::text::AllowShortcuts;
use uefi::proto::device_path::text::DisplayOnly;
use uefi::proto::device_path::DevicePath;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileInfo;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::proto::ProtocolPointer;
use uefi::table::boot::AllocateType;
use uefi::table::boot::LoadImageSource;
use uefi::table::boot::MemoryType;
use uefi::table::boot::ScopedProtocol;
use uefi::table::Boot;
use uefi::table::SystemTable;
use uefi::CStr16;
use uefi::CString16;
use uefi::Char16;
use uefi::Handle;
use uefi::Status;
use uefi_services::println;

mod password_reader;

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status
{
    uefi_services::init(&mut system_table).unwrap();

    println!("Loading the UKI into memory");
    let uki_data = load_file("vmlinuz-linux.unsigned.efi").unwrap();
    println!("Press ENTER to launch the UKI");
    await_enter();
    start_image(uki_data);

    Status::SUCCESS
}

/// Opens the device path protocol and returns a scoped protocol for the device
/// path.
///
/// # Returns
///
/// A `ScopedProtocol` for the `DevicePath`.
fn open_device_path_protocol<'a>() -> ScopedProtocol<'a, DevicePath>
{
    let system_table = unsafe { uefi_services::system_table().as_mut() };
    let image_handle = system_table.boot_services().image_handle();
    let loaded_image = system_table
        .boot_services()
        .open_protocol_exclusive::<LoadedImage>(image_handle)
        .unwrap();
    let device_handle = loaded_image.device();
    let device_path = system_table
        .boot_services()
        .open_protocol_exclusive::<DevicePath>(device_handle)
        .unwrap();
    device_path
}

/// Locates and opens a protocol of type `P`.
///
/// # Type Parameters
///
/// * `P`: The type of protocol pointer to locate and open.
///
/// # Returns
///
/// A `ScopedProtocol` that wraps the opened protocol handle.
fn locate_and_open_protocol<'a, P: ProtocolPointer>() -> ScopedProtocol<'a, P>
{
    let system_table = unsafe { uefi_services::system_table().as_mut() };
    let device_path = open_device_path_protocol();
    let mut device_path = device_path.deref();
    let fs_handle = system_table
        .boot_services()
        .locate_device_path::<P>(&mut device_path)
        .unwrap();
    let opened_handle = system_table
        .boot_services()
        .open_protocol_exclusive(fs_handle)
        .unwrap();
    opened_handle
}

/// Loads a file from the given path and returns a mutable reference to its
/// contents.
///
/// # Arguments
///
/// * `path` - A string slice that holds the path to the file to be loaded.
///
/// # Returns
///
/// An `Option` that contains a mutable reference to the contents of the loaded
/// file if the file was successfully loaded, or `None` otherwise.
fn load_file(path: &str) -> Option<&'static mut [u8]>
{
    let system_table = unsafe { uefi_services::system_table().as_mut() };
    let mut fs = locate_and_open_protocol::<SimpleFileSystem>();
    let mut root = fs.open_volume().unwrap();
    let path = CString16::try_from(path).unwrap();
    let file_handle = root
        .open(&path, FileMode::Read, FileAttribute::empty())
        .ok()?;
    let mut file = file_handle.into_regular_file()?;
    let file_info = file.get_boxed_info::<FileInfo>().unwrap();
    let file_ptr = system_table
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            ((file_info.file_size() as usize - 1) / 4096) + 1,
        )
        .unwrap() as *mut u8;
    unsafe { ptr::write_bytes(file_ptr, 0, file_info.file_size() as usize) };
    let file_buf = unsafe { slice::from_raw_parts_mut(file_ptr, file_info.file_size() as usize) };
    file.read(file_buf).unwrap();

    Some(file_buf)
}

/// Loads and starts an UEFI image from a buffer.
///
/// # Arguments
///
/// * `uki` - A slice of bytes containing the Unified Kernel Image to load.
fn start_image(uki: &[u8])
{
    let system_table = unsafe { uefi_services::system_table().as_mut() };
    let image_handle = system_table.boot_services().image_handle();
    let device_path = open_device_path_protocol();
    let loaded_image = system_table
        .boot_services()
        .load_image(
            image_handle,
            LoadImageSource::FromBuffer {
                buffer: uki,
                file_path: Some(device_path.get().unwrap()),
            },
        )
        .unwrap();
    system_table
        .boot_services()
        .start_image(loaded_image)
        .unwrap();
}

/// Waits for the user to press the Enter key before returning.
fn await_enter()
{
    let system_table = unsafe { uefi_services::system_table().as_mut() };
    loop {
        let mut events = unsafe { [system_table.stdin().wait_for_key_event().unsafe_clone()] };
        system_table
            .boot_services()
            .wait_for_event(&mut events)
            .unwrap();
        let key = system_table.stdin().read_key().unwrap();
        match key {
            Some(Key::Printable(ch)) if char::from(ch) == '\r' => return,
            _ => {}
        }
    }
}
