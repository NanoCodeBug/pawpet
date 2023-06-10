# TODO
- music playback, music file format
- battery consumption tests

## Long Term Todo

Bootloader Custom:
- move hf2 file write logic to bootloader, as well as display rendering logic for status
- require hf2 rust impl that can do flash page writes
- would need larger bootloader segment based on file size tests so far, not clear where extra size is comming from compared to c++ bootloader 

## rust firmware validation
- power save tests again
- measure vcom toggle to verify cycling in suspend and wake

# Board Revision Ideas:

revision with p channel mosfets
- that disconnect display power

revision with ldo and p-channel to disconnect battery
- usb power support, see badger2040 switchover design

revision with simple ni-mh charger
- would need big warnings to only use ni-mh batteries

revision with sepearte daughter board for input, more flexibility in case design

revision with silicon contact-pad style buttons, re use existing buttons or cast own
