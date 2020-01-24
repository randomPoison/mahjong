using UnityEngine;

namespace Synapse.Mahjong
{
    public class PluginTestController : MonoBehaviour
    {
        private void Start()
        {
            var tilesetJson = global::Mahjong.GenerateTilesetJson();
            Debug.Log(tilesetJson, this);
        }
    }
}
