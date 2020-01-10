var LibraryMahjong = {
    /** Converts a JavaScript string to a utf-8 buffer that can be loaded by a Unity script.
     *
     * @param value The string to be converted.
     *
     * @returns A buffer that can be read by the C# runtime.
     */
    $stringToBuffer: function (value) {
        var bufferSize = lengthBytesUTF8(value) + 1;
        var buffer = _malloc(bufferSize);
        stringToUTF8(value, buffer, bufferSize);
        return buffer;
    },

    GenerateTileset: function () {
        // import * as wasm from "mahjong_wasm";

        // var tileset = wasm.generate_tileset();
        // return stringToBuffer(tileset);

        return stringToBuffer("TODO: Actually load the wasm module");
    },
};

autoAddDeps(LibraryMahjong, '$stringToBuffer');
mergeInto(LibraryManager.library, LibraryMahjong);
