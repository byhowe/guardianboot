use alloc::string::String;
use core::fmt::Write;
use core::iter::repeat;

use uefi::proto::console::text::Key;
use uefi::table::Boot;
use uefi::table::SystemTable;
use uefi_services::print;
use uefi_services::system_table;

/// A struct that represents a password reader for UEFI systems.
///
/// This struct is used to read passwords from the user in a UEFI environment.
/// It contains a reference to the UEFI system table, as well as a string to
/// store the password and a boolean to determine whether or not to echo the
/// password as it is typed.
pub struct PasswordReader<'a>
{
    system_table: &'a mut SystemTable<Boot>,
    pwd: String,
    echo: bool,
}

impl PasswordReader<'_>
{
    /// Creates a new instance of PasswordReader.
    ///
    /// # Returns
    ///
    /// A new instance of PasswordReader.
    pub fn new() -> Self
    {
        let system_table = unsafe { system_table().as_mut() };
        PasswordReader {
            system_table,
            pwd: String::new(),
            echo: true,
        }
    }

    /// Reads a password from user input and stores it internallt.
    pub fn read_password(&mut self) -> Result<(), String>
    {
        // Prompt the user to enter their password.
        print!("[guardianboot] Enter password: ");

        // If echo is enabled and the password is not empty, print asterisks to the
        // console.
        if self.echo && self.pwd.len() != 0 {
            self.system_table
                .stdout()
                .write_str(
                    &repeat("*")
                        .take(self.pwd.chars().count())
                        .collect::<String>(),
                )
                .unwrap();
        }

        // Loop until the user finishes entering their password.
        loop {
            let key = self.wait_for_key();

            match key {
                Some(Key::Printable(ch)) => {
                    match char::from(ch) {
                        // Enter key: user finished entering their password.
                        '\r' => {
                            self.system_table.stdout().write_char('\n').unwrap();
                            break;
                        }
                        // Tab key: toggle echo.
                        '\t' => self.toggle_echo(),
                        // Backspace key: remove the last character from the password.
                        '\x08' => self.backspace(),
                        // Any other printable character: add it to the password.
                        ch => self.char_entered(ch),
                    }
                }
                _ => {}
            }
        }

        // Return Ok(()) to indicate that the password was successfully read and stored.
        Ok(())
    }

    /// Returns a reference to the password string.
    pub fn password(&self) -> &str
    {
        &self.pwd
    }

    /// Clears the password string.
    pub fn clear(&mut self)
    {
        self.pwd.clear();
    }

    fn wait_for_key(&mut self) -> Option<Key>
    {
        let event = unsafe {
            self.system_table
                .stdin()
                .wait_for_key_event()
                .unsafe_clone()
        };
        self.system_table
            .boot_services()
            .wait_for_event(&mut [event])
            .unwrap();
        self.system_table.stdin().read_key().unwrap()
    }

    fn toggle_echo(&mut self)
    {
        self.echo = !self.echo;
        let replacement = match self.echo {
            true => "*",
            false => "\x08 \x08",
        };
        let buf = repeat(replacement)
            .take(self.pwd.chars().count())
            .collect::<String>();
        self.system_table.stdout().write_str(&buf).unwrap();
    }

    fn backspace(&mut self)
    {
        if self.pwd.pop().is_some() && self.echo {
            self.system_table.stdout().write_str("\x08 \x08").unwrap();
        }
    }

    fn char_entered(&mut self, ch: char)
    {
        self.pwd.push(ch);
        if self.echo {
            self.system_table.stdout().write_char('*').unwrap();
        }
    }
}
