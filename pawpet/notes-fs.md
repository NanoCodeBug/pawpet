fs todo:

check integrity of entry after write
crc hash it i guess or just read back

if file write fails, attempt at next address
if entry write fails, attemp at next index

cache some number of fs entries to avoid slow lookup times
location of lowest valid block? 
defrag -> rewrite all index entries to end
mount -> search index for lowest valid block+index to start all reads at

bootloader fs update mode
- only expose block erase/read/write
- have updater app handle write failures and other logic

runtime fs mode
- only need read ops for images
- save files still need write/erase though into ring buffer
- do single pass read that builds a list of all file index blocks


## FILESYSTEM NOTES:
- split into to two segments

### game data
- bulk updated via hf2 usb
- wear leveling probably not needed
- bad block detection/mitigation would be ideal
- writes a manifest to the first good blocks with the offsets of the rest of the data

[index] [data data data] [data] [index] [data]
no actual need to split index and data block ordering
its just possible under this model

### index block
- header: INDEX BLOCK
- 16 byte filename
- 4 byte adress of data block
- 4 byte CRC of file

### data block
- contains some number of images

### save files
- ring buffer for even as possible wear leveling
- needs to handle bad block situation and skip to next slot in ring buffer
- on startup search through save file blocks for valid save file
- bad shutdown should show potentially two valid save files, choose first found valid one
- 

write new save -> invalidate old save 
- header: SAVE FILE
- bitmask [valid 1->0, ]
- CRC 
- rtc timestamp
- 

 -- problems: file update speed, if entire data block needs updating, how long will that potentially take over hf2? 20 ms poll at 64 bytes a packet, 3.2 KB/s, 10 minutes to write 2 MB, 60 kb/s at 1ms poll rate. chip can do 1 MB/s write speed in theory if it doesn't need to do erases - experimental, 4.3 kb/s with no render loop and 128 kb r/w buffers, blocker might be disk io methods - specifically block erase method
