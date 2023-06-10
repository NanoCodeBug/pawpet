use hf2;
use hidapi::{DeviceInfo, HidApi};
use std::fs;
use std::io;
use std::io::Read;

fn main() {
    // Statements here are executed when the compiled binary is called.

    // Print text to the console.
    println!("Hello World!");

    let api = HidApi::new().expect("Couldn't find system usb");
    let mut device = None;

    'scanner: loop {
        let devices = api.device_list();

        for d in devices {
            let d = d;

            // println!("{}", d.product_string().unwrap_or_default());
            if d.serial_number().unwrap().starts_with("PAWPET")
                || d.product_string().unwrap().starts_with("PawPet")
            {
                device = Some(d);
                break 'scanner;
            }
            // if d.
            // .open_device(&api).unwrap();
        }

        if device.is_none() {
            println!("no device found, retry?");
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer);
        }
    }

    if device.is_some() {
        let device = device.unwrap();
        println!("{:?}", device);
        println!("{}", device.product_string().unwrap_or_default());
        println!("{}", device.manufacturer_string().unwrap_or_default());
        println!("{}", device.serial_number().unwrap_or_default());

        let device = device.open_device(&api).unwrap();
        // let chk = hf2::checksum_pages(&device, 0x4000, 1);
        let chk = hf2::bin_info(&device);
        println!("{:?}", chk);

        let chk = hf2::info(&device);
        println!("{:?}", chk);

        // let chk = hf2::dmesg(&device);
        // println!("{:?}", chk);

        // let chk = hf2::reset_into_app(&device);
        // println!("{:?}", chk);

        // let chk = hf2::reset_into_bootloader(&device);
        // println!("{:?}", chk);

        let chk = hf2::format_filesystem(&device);
        println!("{:?}", chk);

        let paths = fs::read_dir("../sprites").unwrap();
        let mut found_files = Vec::new();
        let mut total_bytes = 0;
        for path in paths {
            let name = path.as_ref().unwrap().file_name();

            if name.to_str().unwrap_or_default().ends_with(".paw") {
                let path = path.unwrap().path();
                let f = fs::File::open(path.clone()).unwrap();
                let size = f.metadata().unwrap().len();
                total_bytes += size;
                found_files.push(path);
                println!("{:<16} {:4}", name.to_str().unwrap(), size);
            }
        }
        println!("=========================");
        println!("files: {:<3} bytes: {:<6}", found_files.len(), total_bytes);
        println!("=========================");

        for file in found_files {
            let mut f = fs::File::open(file.clone()).unwrap();
            let mut buffer: Vec<u8> = Vec::new();
            f.read_to_end(&mut buffer).expect("failed to read file");
            
            let name = String::from(file.file_name().unwrap().to_str().unwrap());
            let mut split = name.split(".");
            let mut name = String::from(split.next().unwrap());
            name.truncate(16);

            let chk = hf2::send_file(&device, name.as_str(), buffer.clone());
            println!("{:<16} {:?}", name.as_str(), chk);
        }
    }
}
