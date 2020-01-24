using System;
using System.Runtime.InteropServices;
using System.Text;

public class Mahjong
{
    [DllImport("mahjong", EntryPoint = "__cs_bindgen_drop_string", CallingConvention = CallingConvention.Cdecl)]
    private static extern void DropString(IntPtr raw);

    [DllImport("mahjong", EntryPoint = "__cs_bindgen_generated_generate_tileset_json", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr __GenerateTilesetJson(out int length);

    public static string GenerateTilesetJson()
    {
        var rawResult = __GenerateTilesetJson(out var length);
        string result;
        unsafe
        {
            result = Encoding.UTF8.GetString((byte*)rawResult, length);
        }
        DropString(rawResult); return result;
    }

    [DllImport("mahjong", EntryPoint = "__cs_bindgen_generated_tileset_size", CallingConvention = CallingConvention.Cdecl)]
    private static extern uint __TilesetSize();

    public static uint TilesetSize()
    {
        var rawResult = __TilesetSize();
        return rawResult;
    }
}
