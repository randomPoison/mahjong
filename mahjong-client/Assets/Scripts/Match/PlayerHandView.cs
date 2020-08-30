using Synapse.Utils;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using UnityEngine;

namespace Synapse.Mahjong.Match
{
    /// <summary>
    /// Shared logic for managing the visual state of a player's hand.
    /// </summary>
    ///
    /// <remarks>
    /// <para>
    /// This class has the core logic for managing the game objects representing the tiles in a
    /// players hand. Most of its functionality is kept <c>protected</c> with the expectation that
    /// the derived class will re-expose that functionality in a more appropriate way.
    /// </para>
    ///
    /// <para>
    /// For the concrete player hand view logic, see <see cref="LocalHandView"/> and
    /// <see cref="RemoteHandView"/>.
    /// </para>
    /// </remarks>
    public abstract class PlayerHandView : MonoBehaviour
    {
        #region Constants

        // TODO: Figure out a better way to track tile dimensions. This should likely be
        // tracked along with the tile asset set, once we move the tile set to a custom
        // asset.
        public const float TileWidth = 0.026f;
        public const float TileLength = 0.034f;
        public const float MeldSpacing = 0.01f;

        #endregion

        #region Configuration Fields

        [SerializeField] private Transform _handRoot = default;
        [SerializeField] private Transform _drawTileAnchor = default;
        [SerializeField] private Transform _discardRoot = default;
        [SerializeField] private Transform _meldRoot = default;

        #endregion

        #region Private Fields

        private List<GameObject> _tiles = new List<GameObject>();
        private GameObject _currentDraw = null;
        private List<TileView> _discards = new List<TileView>();
        private List<List<TileView>> _melds = new List<List<TileView>>();

        #endregion

        #region Properties

        public ReadOnlyCollection<TileView> Discards => _discards.AsReadOnly();

        public bool HasCurrentDraw => _currentDraw != null;

        #endregion

        public TileView RemoveLastDiscard()
        {
            var removed = _discards[_discards.Count - 1];
            _discards.RemoveAt(_discards.Count - 1);

            return removed;
        }

        protected void AddTile(GameObject tile)
        {
            _tiles.Add(tile);
            LayoutHand();
        }

        protected void AddTiles(IEnumerable<GameObject> tiles)
        {
            _tiles.AddRange(tiles);
            LayoutHand();
        }

        protected void AddDrawTile(GameObject tile)
        {
            tile.transform.SetParent(_drawTileAnchor, false);
            tile.transform.localPosition = Vector3.zero;
            tile.transform.localRotation = Quaternion.identity;

            _currentDraw = tile;
        }

        /// <summary>
        /// Removes a tile from the player's hand.
        /// </summary>
        ///
        /// <param name="index">The index of the tile to remove.</param>
        ///
        /// <returns>The game object for the tile.</returns>
        ///
        /// <remarks>
        /// The tile is removed both the hand's internal tracking and from the root
        /// object's list of children.
        /// </remarks>
        protected GameObject RemoveFromHand(int index)
        {
            var tile = _tiles[index];
            _tiles.RemoveAt(index);

            tile.transform.SetParent(null, false);

            return tile;
        }

        protected GameObject RemoveCurrentDraw()
        {
            var currentDraw = _currentDraw;
            _currentDraw = null;

            currentDraw.transform.SetParent(null, false);

            return currentDraw;
        }

        protected void AddDiscard(TileView discarded)
        {
            // Add the discarded tile to the list of discards.
            _discards.Add(discarded);

            // Make the discarded tile a child of the root object for the discard pile,
            // but keep its world position so that we can animate it from its current
            // position to its target position int the discard pile.
            discarded.transform.SetParent(_discardRoot, worldPositionStays: true);

            LayoutHand();
        }

        protected void AddMeld(List<TileView> meld)
        {
            _melds.Add(meld);

            // Make all of the tile view objects children of the meld root object.
            //
            // TODO: We may eventually want to give each meld its own dedicated root
            // object to make layout easier, but for now we'll make all of meld tiles
            // direct children of the meld root.
            foreach (var view in meld)
            {
                view.transform.SetParent(_meldRoot, false);
            }

            LayoutHand();
        }

        private void LayoutHand()
        {
            // Layout tiles in the player's hand.
            {
                var leftSide = _tiles.Count * -TileWidth * 0.5f;
                foreach (var (index, tileObj) in _tiles.Enumerate())
                {
                    tileObj.transform.SetParent(_handRoot);
                    tileObj.transform.localRotation = Quaternion.identity;
                    tileObj.transform.localPosition = new Vector3(
                        leftSide + TileWidth * index,
                        0f,
                        0f);
                }
            }

            // Layout the discarded tiles in rows of 6 tiles.
            {
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
            }

            // Layout the melds. For now we layout the tiles in the meld left-to-right
            // with all tiles in the meld adjacent to each other and with a small gap
            // between the melds.
            //
            // TODO: Improve meld visualization:
            //
            // * Rotate the tile that was called.
            // * Display melds in multiple rows if necessary to make better use of space.
            {
                var tilePos = 0f;
                foreach (var meld in _melds)
                {
                    foreach (var tile in meld)
                    {
                        tile.transform.localPosition = new Vector3(
                            tilePos,
                            0f,
                            TileLength * 0.5f);
                        tile.transform.localRotation = Quaternion.identity;

                        tilePos += TileWidth;
                    }

                    tilePos += MeldSpacing;
                }
            }
        }
    }
}
