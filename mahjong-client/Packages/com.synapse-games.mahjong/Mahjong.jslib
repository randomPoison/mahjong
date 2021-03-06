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
        // TODO: Is `generate_tileset` actually a global symbol at this point?
        var tileset = generate_tileset();
        return stringToBuffer(tileset);
    },
};

autoAddDeps(LibraryMahjong, '$stringToBuffer');
mergeInto(LibraryManager.library, LibraryMahjong);
