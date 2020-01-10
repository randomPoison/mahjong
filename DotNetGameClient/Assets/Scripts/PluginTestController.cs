using System.Runtime.InteropServices;
using UnityEngine;

namespace Synapse.Mahjong
{
    public class PluginTestController : MonoBehaviour
    {
        // TODO: Move the plugin interop into its own class.
        [DllImport("__Internal", EntryPoint = "generate_tileset")]
        public static extern string GenerateTileset();

        private void Start()
        {
            var tilesetJson = GenerateTileset();
            Debug.Log($"Tileset JSON: {tilesetJson}");
        }
    }
}
