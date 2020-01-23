using System;
using System.Runtime.InteropServices;
using System.Text;
using UnityEngine;

namespace Synapse.Mahjong
{
    public class Mahjong : MonoBehaviour
    {
        [DllImport(
            "mahjong",
            EntryPoint = "__cs_bindgen_generated_generate_tileset_json",
            CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr GenerateTilesetJson(out int length);

        [DllImport(
            "mahjong",
            EntryPoint = "__cs_bindgen_drop_string",
            CallingConvention = CallingConvention.Cdecl)]
        private static extern void DropString(IntPtr raw);

        public string GenerateTileset()
        {
            var rawPtr = GenerateTilesetJson(out var length);

            string result;
            unsafe
            {
                result = Encoding.UTF8.GetString((byte*)rawPtr, length);
            }

            DropString(rawPtr);

            return result;
        }
    }
}
