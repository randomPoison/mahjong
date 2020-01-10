using UnityEngine;

namespace Synapse.Mahjong
{
    public class PluginTestController : MonoBehaviour
    {
        private void Start()
        {
            var tilesetJson = Mahjong.GenerateTileset();
            Debug.Log($"Tileset JSON: {tilesetJson}");
        }
    }
}
