using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using Synapse.Utils;
using UniRx.Async;
using UnityEngine;

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
        private const float TileLength = 0.034f;

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
        private List<TileView> _discards = new List<TileView>();

        private List<GameObject> _dummyTiles = new List<GameObject>();
        private GameObject _dummyCurrentDraw = null;

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

        #region Local hand

        public void AddToHand(TileView tile)
        {
            // Add the tile to the internal state tracking for the hand.
            _tiles.Add(tile);
            tile.Clicked += OnTileClicked;

            // Make the tile object a child of the root object for the tiles in the
            // player's hand.
            tile.transform.SetParent(_handRoot, worldPositionStays: false);

            LayoutHand(_tiles.Select(view => view.gameObject));
        }

        public async UniTask DrawTile(TileView tile)
        {
            Debug.Assert(
                _currentDraw == null,
                $"Adding draw tile {tile.Model} to player hand {this} when player " +
                $"already has draw tile");

            _currentDraw = tile;
            tile.Clicked += OnTileClicked;

            tile.transform.SetParent(_drawTileAnchor, worldPositionStays: false);
            tile.transform.localPosition = Vector3.zero;

            // TODO: Animate the draw action. This delay is just here as a placeholder
            // to ensure the code handles the delay that will eventually be here once we
            // implement an animation.
            await UniTask.Delay(500);
        }

        public void MoveToDiscard(TileId id)
        {
            TileView discarded;

            // Get the `TileView` object for the selected tile, either from the tiles
            // in the player's hand or from the current draw.
            var discardIndex = _tiles.FindIndex(tile => tile.Model.Id.Element0 == id.Element0);
            if (discardIndex >= 0)
            {
                discarded = _tiles[discardIndex];
                _tiles.RemoveAt(discardIndex);
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

            // Add the discarded tile to the list of discards.
            _discards.Add(discarded);

            // Make the discarded tile a child of the root object for the discard pile,
            // but keep its world position so that we can animate it from its current
            // position to its target position int the discard pile.
            discarded.transform.SetParent(_discardRoot, worldPositionStays: true);

            // TODO: Actually do a tween. For now we'll immediately display the tile in
            // the player's discards.

            // Layout the discarded tiles in rows of 6 tiles.
            var leftSide = TileWidth * -6 * 0.5f;
            foreach (var (index, tile) in _discards.Enumerate())
            {
                int row = index / 6;
                int col = index % 6;
                tile.transform.localPosition = new Vector3(
                    leftSide + col * TileWidth,
                    0f,
                    -row * TileLength);

                tile.transform.localRotation = Quaternion.identity;
            }

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

        public UniTask<TileId> OnClickTileAsync(CancellationToken cancellation = default)
        {
            var completion = new UniTaskCompletionSource<TileId>();
            TileClicked += Handler;
            cancellation.Register(() =>
            {
                completion.TrySetCanceled();
            });
            return completion.Task;

            void Handler(PlayerHand hand, TileId id)
            {
                completion.TrySetResult(id);
                TileClicked -= Handler;
            }
        }

        #endregion

        #region Remote hand

        public void FillWithDummyTiles(GameObject prefab)
        {
            for (var count = 0; count < 13; count += 1)
            {
                _dummyTiles.Add(Instantiate(prefab, _handRoot));
            }

            LayoutHand(_dummyTiles);
        }

        public async UniTask DrawDummyTile()
        {
            throw new NotImplementedException();

            // TODO: Animate the draw action. This delay is just here as a placeholder
            // to ensure the code handles the delay that will eventually be here once we
            // implement an animation.
            await UniTask.Delay(500);
        }

        #endregion

        #region Layout Logic

        private void LayoutHand(IEnumerable<GameObject> tiles)
        {
            var leftSide = tiles.Count() * -TileWidth * 0.5f;
            foreach (var (index, tileObj) in tiles.Enumerate())
            {
                tileObj.transform.localPosition = new Vector3(
                    leftSide + TileWidth * index,
                    0f,
                    0f);
            }
        }

        #endregion

        #region Event Handlers

        private void OnTileClicked(TileView clicked)
        {
            TileClicked?.Invoke(this, clicked.Model.Id);
        }

        #endregion
    }
}
