using UnityEngine;

namespace Synapse.Mahjong.Match
{
    /// <summary>
    /// Controller script for a player's hand during a mahjong match.
    /// </summary>
    public class PlayerHand : MonoBehaviour
    {
        [SerializeField] private Transform _handRoot = default;
        [SerializeField] private Transform _drawTileAnchor = default;

        public Transform HandRoot => _handRoot;
        public Transform DrawTileAnchor => _drawTileAnchor;
    }
}
