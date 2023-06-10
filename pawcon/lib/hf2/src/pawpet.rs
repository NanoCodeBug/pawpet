use crate::command::{rx, xmit, Command};
use crate::Error;
use scroll::Pwrite;

///Dual of READ WORDS, with the same constraints. Empty tuple response.
pub fn send_file(
    d: &hidapi::HidDevice,
    name: &str,
    bytes: Vec<u8>,
) -> Result<(), Error> {
    let mut buffer = vec![0_u8; bytes.len() +  16];
    let mut offset = 0;

    assert!(name.len() <= 16);

    for i in 0..16 {
        if i < name.len()
        {
            buffer.gwrite_with(name.as_bytes()[i], &mut offset, scroll::LE)?;
        }
        else {
            let b: u8 = 0;
            buffer.gwrite_with(b, &mut offset, scroll::LE)?;
        }
    }

    for i in bytes {
        buffer.gwrite_with(i, &mut offset, scroll::LE)?;
    }

    xmit(Command::new(0xff68, 0, buffer), d)?;

    rx(d).map(|_| ())
}


///Dual of READ WORDS, with the same constraints. Empty tuple response.
pub fn format_filesystem(
    d: &hidapi::HidDevice,

) -> Result<(), Error> {

    xmit(Command::new(0xba8e, 0, vec![]), d)?;

    rx(d).map(|_| ())
}
