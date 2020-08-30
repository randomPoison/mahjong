using System;
using System.Collections.Generic;
using System.Threading;
using UniRx.Async;
using UnityEngine;

namespace Synapse.Mahjong.Match
{
    public sealed class LocalHandView : PlayerHandView
    {
        private List<TileView> _tileViews = new List<TileView>();
        private TileView _drawView = null;

        #region Events

        /// <summary>
        /// Event broadcast when the player clicks on a tile.
        /// </summary>
        public event Action<PlayerHandView, TileId> TileClicked;

        #endregion

        public void AddToHand(TileView tile)
        {
            // Add the tile to the internal state tracking for the hand.
            _tileViews.Add(tile);
            tile.Clicked += OnTileClicked;

            AddTile(tile.gameObject);
        }

        public async UniTask DrawTile(TileView tile)
        {
            Debug.Assert(
                _drawView == null,
                $"Adding draw tile {tile.Model} to player hand {this} when player " +
                $"already has draw tile");

            _drawView = tile;
            tile.Clicked += OnTileClicked;

            AddDrawTile(tile.gameObject);

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
            var discardIndex = FindIndex(id);
            if (discardIndex >= 0)
            {
                discarded = _tileViews[discardIndex];
                _tileViews.RemoveAt(discardIndex);

                // Remove the tile object from the underlying view data.
                RemoveFromHand(discardIndex);
            }
            else if (_drawView != null && _drawView.Model.Id.Element0 == id.Element0)
            {
                discarded = _drawView;
                _drawView = null;

                // Remove the tile object from the underlying view data.
                RemoveCurrentDraw();
            }
            else
            {
                throw new ArgumentException($"Tile {id} is not in player's hand");
            }

            // Remove the click handler so that we don't get click events from discarded
            // tiles.
            discarded.Clicked -= OnTileClicked;

            // Add the discarded tile to the list of discards.
            AddDiscard(discarded);

            // If we didn't discard the drawn tile, merge the drawn tile into the
            // player's hand.
            if (_drawView != null)
            {
                RemoveCurrentDraw();
                AddToHand(_drawView);
                _drawView = null;
            }
        }

        public void CallTile(TileView discard, ICall call)
        {
            // Remove other tiles that are part of the meld from the player's hand.
            switch (call)
            {
                case Call.Chii chii:
                {
                    var meld = new List<TileView>
                    {
                        discard,
                        RemoveFromHand(chii.Element0),
                        RemoveFromHand(chii.Element1),
                    };
                    AddMeld(meld);
                }
                break;

                case Call.Pon pon:
                {
                    throw new NotImplementedException();
                }

                case Call.Kan kan:
                {
                    throw new NotImplementedException();
                }

                case Call.Ron ron:
                {
                    throw new NotImplementedException();
                }

                default:
                    throw new NotImplementedException($"Unhandled call: {call}");
            }

            // TODO: Add the completed meld to the player's open melds.
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

            void Handler(PlayerHandView hand, TileId id)
            {
                completion.TrySetResult(id);
                TileClicked -= Handler;
            }
        }

        /// <summary>
        /// Removes the tile with the specified ID from the hand.
        /// </summary>
        ///
        /// <param name="id">The ID of the tile to remove.</param>
        ///
        /// <returns>
        /// The view object for the specified tile.
        /// </returns>
        private TileView RemoveFromHand(TileId id)
        {
            var index = FindIndex(id);
            var view = _tileViews[index];

            // Remove the tile view from our local tracking.
            _tileViews.RemoveAt(index);

            // Remove the game object from the underlying hand view.
            RemoveFromHand(index);

            return view;
        }

        /// <summary>
        /// Returns the index of the specified tile ID.
        /// </summary>
        ///
        /// <param name="id">The ID of the tile to find.</param>
        ///
        /// <returns>
        /// The index of the tile within the player's hand, or -1 if the tile is not in
        /// the player's hand.
        /// </returns>
        ///
        /// <remarks>
        /// Only the player's main hand is searched, so this will return -1 even if the
        /// tile is the player's current draw, in the player's discards, or in an open
        /// meld.
        /// </remarks>
        private int FindIndex(TileId id)
        {
            return _tileViews.FindIndex(tile => tile.Model.Id.Element0 == id.Element0);
        }

        #region Event Handlers

        private void OnTileClicked(TileView clicked)
        {
            TileClicked?.Invoke(this, clicked.Model.Id);
        }

        #endregion
    }
}
