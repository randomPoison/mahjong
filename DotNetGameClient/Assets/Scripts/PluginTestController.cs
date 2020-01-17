using UnityEngine;

namespace Synapse.Mahjong
{
    public class PluginTestController : MonoBehaviour
    {
        [SerializeField]
        private Mahjong _plugin;

        private void Start()
        {
            var tilesetJson = _plugin.GenerateTileset();
            Debug.Log($"Tileset JSON: {tilesetJson}");
        }
    }
}
