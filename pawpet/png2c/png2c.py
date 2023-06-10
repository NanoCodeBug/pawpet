from PIL import Image
from enum import IntEnum, unique
import os
import sys
import re


@unique
class Colors(IntEnum):
    WHITE = 0
    BLACK = 1
    CLEAR = 2
    NULL = 3

# 2 bit packing, 4 pixels per byte
# byte [00 00 00 00]

# 1 bit packing, 8 pixels per byte
# byte [0 0 0 0 0 0 0 0]


def packPixels(image, supportAlpha, outBytes):
    currentByte = 0
    currentPixesInByte = 0
    maxPixelsInByte = 4 if supportAlpha else 8

    for x in list(image.getdata()):
        p = getColor(x)

        if not supportAlpha:
            if p == Colors.CLEAR:
                p = Colors.WHITE
        currentByte = currentByte | (
            p << currentPixesInByte * (2 if supportAlpha else 1))
        currentPixesInByte += 1

        if currentPixesInByte == maxPixelsInByte:
            assert currentByte < 256
            outBytes.append(currentByte)
            currentPixesInByte = 0
            currentByte = 0

    # clean up loose pixels
    if currentPixesInByte > 0:
        assert currentByte < 256
        outBytes.append(currentByte)
# 2 bit color, 7 bits span length
# 1 bit color, 7 bits span length
# span ordering
# [3 2 1 0] [7 6 5 4]


def spanPixels(image, supportAlpha, outBytes):
    currentByte = 0
    currentColor = Colors.NULL
    spanCount = 0
    maxSpan = 63 if supportAlpha else 127

    spanIndex = 0

    for x in list(image.getdata()):
        p = getColor(x)

        if not supportAlpha:
            if p == Colors.CLEAR:
                p = Colors.WHITE

        if currentColor == p:
            # coninue to pack span
            if spanCount >= maxSpan:
                # no space in span, start new span with current pixel in it
                currentByte = packSpanByte(
                    currentByte, currentColor, spanCount, 0, supportAlpha)

                outBytes.append(currentByte)
                currentByte = 0

                spanCount = 1
            else:
                spanCount += 1

        elif currentColor != p:
            # span changed, pack and start new span
            if spanCount > 0:
                currentByte = packSpanByte(
                    currentByte, currentColor, spanCount, 0, supportAlpha)

                outBytes.append(currentByte)
                currentByte = 0

            spanCount = 1
            currentColor = p

    # clean up trailing span
    currentByte = packSpanByte(
        currentByte, currentColor, spanCount, 0, supportAlpha)

    if spanCount > 0:
        outBytes.append(currentByte)


def packSpanByte(currentData, color, spanSize, index, alpha):
    if not alpha:
        currentData |= ((color << 7) | spanSize) << (index * 8)
    else:
        currentData |= ((color << 6) | spanSize) << (index * 8)
    return currentData


def getEncodingBitCount(rawBytes):
    foundClear = False
    foundWhite = False

    for x in rawBytes:
        val = getColor(x)
        # check alpha channel
        if val == Colors.CLEAR:
            foundClear = True
        elif val == Colors.WHITE:
            foundWhite = True

        if(foundWhite and foundClear):
            return 2

    return 1


def getColor(x):
    # check alpoha channel if present
    if len(x) > 3 and x[3] < 128:
        return Colors.CLEAR
    # check red channel
    elif x[0] < 128:
        return Colors.BLACK
    else:
        return Colors.WHITE


def toBytes(graphic):
    imageBytes = bytearray()

    # add metadata to front of array
    imageBytes.extend(graphic["width"].to_bytes(2, byteorder='little'))
    imageBytes.extend(graphic["height"].to_bytes(2, byteorder='little'))

    encoding = 0
    if graphic["encoding"] == "span":
        encoding = 1
    alpha = (1 if graphic["alpha"] else 0)

    encoding = encoding << 1
    encoding |= alpha
    imageBytes.extend(encoding.to_bytes(2, byteorder='little'))

    imageBytes.extend(graphic["tile_count"].to_bytes(2, byteorder='little'))

    if ("frame_offsets" in graphic):
        for x in range(0, len(graphic["frame_offsets"])):
            x1 = graphic["frame_offsets"][x]

            imageBytes.extend(x1.to_bytes(2, byteorder='little'))

    for x in graphic["bytes"]:
        assert x < 256
        imageBytes.extend(x.to_bytes(1, byteorder='little'))

    return imageBytes



def writeCustomBmp(graphic, serializedBytes, path):
    f = open(path + graphic["name"] + ".paw", "wb")

    f.write(serializedBytes)
    f.close()

def main():
    if len(sys.argv) != 3:
        print("usage: png2c [input_directory] [output_build_files]")
        print("expected naming:")
        print("<filename> - single image, dimensions automatically detected")
        print()
        print("<filename>_<width>x<height> - rectangle or square tiles of size <width> and <height>, example 'spritemap_8_16', tile size of 8x16 sprite map")
        print()
        print("order of tiles starts with top-left being 0, left->right incrementing per row")

    directory = sys.argv[1]
    buildpath = sys.argv[2]

    pngFiles = list()
    for entry in os.scandir(directory):
        if (entry.path.endswith(".png")) and entry.is_file():
            pngFiles.append(entry.path)

    totalBytes = 0

    for imageFile in pngFiles:
        image = Image.open(imageFile)

        petGraphic = {}
        petGraphic["bytes"] = list()
        petGraphic["alpha"] = (getEncodingBitCount(list(image.getdata())) > 1)

        nameTemp = os.path.splitext(os.path.basename(imageFile))[0]
        isSpriteMap = re.search('(.*)_([0-9]+)x*([0-9]*)$', nameTemp)

        if (isSpriteMap):
            tileWidth = 0
            tileHeight = 0
            # square sprite map
            if (isSpriteMap.group(2) != ""):
                tileWidth = int(isSpriteMap.group(2))
                tileHeight = int(isSpriteMap.group(2))

            # non-square sprite map
            if (isSpriteMap.group(3) != ""):
                tileHeight = int(isSpriteMap.group(3))

            if(image.width % tileWidth != 0):
                print("WARN: tile width not multiple of image width")
            if(image.height % tileHeight != 0):
                print("WARN: tile height not multiple of image height")

            petGraphic["width"] = tileWidth
            petGraphic["height"] = tileHeight
            petGraphic["tile_count"] = int(image.width / tileWidth) * int(image.height / tileHeight)
            petGraphic["map_width"] = mapWidth = int(image.width / tileWidth)
            petGraphic["map_height"] = mapHeight = int( image.height / tileHeight)
            petGraphic["name"] = isSpriteMap.group(1)
            petGraphic["frame_offsets"] = list()

            packBytes = list()
            spanBytes = list()

            packOffsets = list()
            spanOffsets = list()

            for y in range(0, mapHeight):
                for x in range(0, mapWidth):
                    sprite = image.crop(
                        (x * tileWidth, y * tileHeight, x * tileWidth + tileWidth, y * tileHeight + tileHeight))

                    packOffsets.append(len(packBytes))
                    packPixels(sprite, petGraphic["alpha"], packBytes)

                    spanOffsets.append(len(spanBytes))
                    spanPixels(sprite, petGraphic["alpha"], spanBytes)

            if len(spanBytes) > len(packBytes):
                petGraphic["encoding"] = "pack"
                petGraphic["bytes"] = packBytes
                petGraphic["frame_offsets"] = packOffsets
            else:
                petGraphic["encoding"] = "span"
                petGraphic["bytes"] = spanBytes
                petGraphic["frame_offsets"] = spanOffsets

        # not a sprite map
        else:
            petGraphic["width"] = image.width
            petGraphic["height"] = image.height
            petGraphic["name"] = nameTemp
            petGraphic["tile_count"] = 1
            # pack single images pixel data
            packBytes = list()
            spanBytes = list()

            packPixels(image, petGraphic["alpha"], packBytes)
            spanPixels(image, petGraphic["alpha"], spanBytes)

            if len(spanBytes) > len(packBytes):
                petGraphic["encoding"] = "pack"
                petGraphic["bytes"] = packBytes
            else:
                petGraphic["encoding"] = "span"
                petGraphic["bytes"] = spanBytes

        print("{0} -> span:{1}, pack:{2}".format(petGraphic["name"], len(spanBytes), len(packBytes)))

        serializedBytes = toBytes(petGraphic)
        totalBytes += len(serializedBytes)

        print("{0} - {1}x{2} bytes:{3}, alpha:{4}, tiles:{5}, encoding:{6}".format(petGraphic["name"], petGraphic["width"], petGraphic["height"], len(
            petGraphic["bytes"]), petGraphic["alpha"], petGraphic["tile_count"], petGraphic["encoding"]))

        writeCustomBmp(petGraphic, serializedBytes, buildpath)


    print("total bytes:", totalBytes)


if __name__ == "__main__":
    main()
