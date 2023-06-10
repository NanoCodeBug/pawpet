use pawbsp as bsp;
use bsp::pac::ADC;
use bsp::hal;

use hal::adc::Adc;
use hal::prelude::*;
use hal::prelude::_atsamd_hal_embedded_hal_digital_v2_OutputPin;

pub struct BatteryMonitor {
    bat_enable: bsp::EnableVBAT,
    bat_read: bsp::InputVBAT,
    adc: Adc<ADC>,
}

impl pawdevicetraits::BatteryMonitorDevice for BatteryMonitor
{
    fn read(&mut self) -> u16 {
        self.bat_enable.set_low().unwrap();

        let mut val: u32 = self.adc.read(&mut self.bat_read).unwrap();
        val = val * 2 * 2 * 1000 / 4096;
        // external resistor dividor is 2 (3.4 vmax -> 1.7)
        // input voltage is divided by 2 (1.7 -> 0.85)
        // internal reference voltage is 1, (0.85 / 1.0)
        // 1000 mult to keep integer math instead of float
        // 4096 is the value range of the 12 bit read
        // total voltage read range of 0.00v -> 4.00 volts, 0 -> 4000
        // divided by 10 to remove noise from last digit, now as a 0 -> 400

        self.bat_enable.set_high().unwrap();
        return val as u16 / 10;
    }
}

impl BatteryMonitor {
    pub fn new(
        mut adc: Adc<ADC>,
        mut bat_enable: bsp::EnableVBAT,
        bat_read: bsp::InputVBAT,
    ) -> Self {
        bat_enable.set_high().unwrap(); // disconnect battery read to minimize power draw

        adc.resolution(hal::adc::Resolution::_12BIT);
        adc.gain(hal::adc::Gain::DIV2);
        adc.reference(hal::adc::Reference::INT1V);
        adc.samples(hal::adc::SampleRate::_8);

        Self {
            bat_enable,
            bat_read,
            adc,
        }
    }
}
