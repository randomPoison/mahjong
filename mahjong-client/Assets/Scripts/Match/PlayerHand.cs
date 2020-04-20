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
    public class PlayerHand : MonoBehaviour, IPointerClickHandler
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

        // TODO: Would it make more sense to instead have a reference to the `Player`
        // object? It already tracks the tiles in the player's hand and their current
        // draws. If we did that, this class would be more focused on tracking just the
        // front-end state. We would likely need better support for optional/nullable
        // types in cs-bindgen in order to make that approach ergonomic.
        private List<TileInstance> _tiles = new List<TileInstance>();
        private TileInstance? _currentDraw = null;

        private List<GameObject> _tileObjects = new List<GameObject>();
        private GameObject _currentDrawObject = null;

        #endregion

        public void AddToHand(TileInstance tile, GameObject tileObject)
        {
            // Add the tile to the internal state tracking for the hand.
            //
            // TODO: Would it be better to have a script on the tile object that also
            // tracks its corresponding tile instance? As it stands we're basically
            // always going to have to pass around the tile instance + game object pair.
            _tiles.Add(tile);
            _tileObjects.Add(tileObject);
            Debug.Assert(
                _tiles.Count == _tileObjects.Count,
                "Tile list and game object list are out of sync");

            // Make the tile object a child of the root object for the tiles in the
            // player's hand.
            tileObject.transform.SetParent(_handRoot, false);

            // Re-layout the updated set of tiles in the player's hand.
            var leftSide = _tileObjects.Count * -TileWidth * 0.5f;
            foreach (var (index, tileObj) in _tileObjects.Enumerate())
            {
                tileObj.transform.localPosition = new Vector3(
                    leftSide + TileWidth * index,
                    0f,
                    0f);
            }
        }

        public void DrawTile(TileInstance tile, GameObject tileObject)
        {
            _currentDraw = tile;
            _currentDrawObject = tileObject;

            _currentDrawObject.transform.SetParent(_drawTileAnchor, false);
            _currentDrawObject.transform.localPosition = Vector3.zero;
        }

        void IPointerClickHandler.OnPointerClick(PointerEventData eventData)
        {
            throw new NotImplementedException();
        }
    }
}
