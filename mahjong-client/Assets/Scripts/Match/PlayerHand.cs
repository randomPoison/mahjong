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

        [SerializeField] private Wind _seat = default;
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
        public event Action<PlayerHand, TileId> TileClicked;

        #endregion

        #region Properties

        public Wind Seat => _seat;

        #endregion

        public void AddToHand(TileView tile)
        {
            // Add the tile to the internal state tracking for the hand.
            _tiles.Add(tile);
            tile.Clicked += OnTileClicked;

            // Make the tile object a child of the root object for the tiles in the
            // player's hand.
            tile.transform.SetParent(_handRoot, worldPositionStays: false);

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

            tile.transform.SetParent(_drawTileAnchor, worldPositionStays: false);
            tile.transform.localPosition = Vector3.zero;
        }

        public void MoveToDiscard(TileId id)
        {
            TileView discarded;

            // Get the `TileView` object for the selected tile, either from the tiles
            // in the player's hand or from the current draw.
            var index = _tiles.FindIndex(tile => tile.Model.Id.Element0 == id.Element0);
            if (index >= 0)
            {
                discarded = _tiles[index];
                _tiles.RemoveAt(index);
            }
            else if (_currentDraw != null && _currentDraw.Model.Id.Element0 == id.Element0)
            {
                discarded = _currentDraw;
                _currentDraw = null;
            }
            else
            {
                throw new ArgumentException($"Tile {id} is not in {Seat} player's hand");
            }

            // Make the discarded tile a child of the root object for the discard pile,
            // but keep its world position so that we can animate it from its current
            // position to its target position int the discard pile.
            //
            // TODO: Actually do that tween.
            discarded.transform.SetParent(_discardRoot, worldPositionStays: true);

            // Remove the click handler so that we don't get click events from discarded
            // tiles.
            discarded.Clicked -= OnTileClicked;

            // If we didn't discard the drawn tile, merge the drawn tile into the
            // player's hand.
            if (_currentDraw != null)
            {
                AddToHand(_currentDraw);
                _currentDraw = null;
            }
        }

        #region Event Handlers

        private void OnTileClicked(TileView clicked)
        {
            TileClicked?.Invoke(this, clicked.Model.Id);
        }

        #endregion
    }
}
