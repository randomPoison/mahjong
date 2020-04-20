using System;
using System.Collections.Generic;
using Synapse.Utils;
using UnityEngine;
using UnityEngine.EventSystems;

namespace Synapse.Mahjong.Match
{
    /// <summary>
    /// Controller script for a player's hand during a mahjong match.
    /// </summary>
    public class PlayerHand : MonoBehaviour
    {
        #region Constants

        // TODO: Figure out a better way to track tile dimensions. This should likely be
        // tracked along with the tile asset set, once we move the tile set to a custom
        // asset.
        private const float TileWidth = 0.026f;

        #endregion

        #region Configuration Fields
        
        [SerializeField] private Transform _handRoot = default;
        [SerializeField] private Transform _drawTileAnchor = default;
        [SerializeField] private Transform _discardRoot = default;

        #endregion

        #region Private Fields

        private List<TileView> _tiles = new List<TileView>();
        private TileView _currentDraw = null;

        #endregion

        #region Events

        /// <summary>
        /// Event broadcast when the player clicks on a tile.
        /// </summary>
        public event Action<TileId> TileClicked;

        #endregion

        public void AddToHand(TileView tile)
        {
            // Add the tile to the internal state tracking for the hand.
            _tiles.Add(tile);
            tile.Clicked += OnTileClicked;

            // Make the tile object a child of the root object for the tiles in the
            // player's hand.
            tile.transform.SetParent(_handRoot, false);

            // Re-layout the updated set of tiles in the player's hand.
            var leftSide = _tiles.Count * -TileWidth * 0.5f;
            foreach (var (index, tileObj) in _tiles.Enumerate())
            {
                tileObj.transform.localPosition = new Vector3(
                    leftSide + TileWidth * index,
                    0f,
                    0f);
            }
        }

        public void DrawTile(TileView tile)
        {
            Debug.Assert(
                _currentDraw == null,
                $"Adding draw tile {tile.Model} to player hand {this} when player " +
                $"already has draw tile");
            _currentDraw = tile;
            tile.Clicked += OnTileClicked;

            tile.transform.SetParent(_drawTileAnchor, false);
            tile.transform.localPosition = Vector3.zero;
        }

        void OnTileClicked(TileView clicked)
        {
            TileClicked?.Invoke(clicked.Model.Id);
        }
    }
}
