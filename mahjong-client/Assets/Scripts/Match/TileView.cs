using System;
using UnityEngine;
using UnityEngine.EventSystems;

namespace Synapse.Mahjong.Match
{
    /// <summary>
    /// View controller for a single tile object in the scene.
    /// </summary>
    public class TileView : MonoBehaviour, IPointerClickHandler
    {
        /// <summary>
        /// Emits an event when the player clicks on this tile. Parameter is the tile
        /// view object that was clicked.
        /// </summary>
        public Action<TileView> Clicked;

        /// <summary>
        /// The data model for this tile. Specifies the unique ID for the tile and the
        /// tile's value.
        /// </summary>
        public TileInstance Model { get; private set; }

        /// <summary>
        /// Initialize the tile with a data model.
        /// </summary>
        ///
        /// <param name="model">The data model for the tile.</param>
        ///
        /// <remarks>
        /// Also resets callback state for the view object. This allows view objects to
        /// be pooled and re-populated with new IDs as needed.
        /// </remarks>
        // TODO: We should really only need to initialize the tile with its ID. The
        // value of the tile should be known ahead of time when the prefab is setup.
        // If we setup the value on the prefab we could also use that at runtime to
        // assert that the data model's tile value matches the prefabs value.
        public void Populate(TileInstance model)
        {
            Model = model;
            Clicked = null;
        }

        #region Input Handlers

        void IPointerClickHandler.OnPointerClick(PointerEventData eventData)
        {
            Clicked?.Invoke(this);
        }

        #endregion
    }
}
