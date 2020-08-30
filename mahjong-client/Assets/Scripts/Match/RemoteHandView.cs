using System;
using System.Collections.Generic;
using System.Linq;
using UniRx.Async;
using UnityEngine;

namespace Synapse.Mahjong.Match
{
    public sealed class RemoteHandView : PlayerHandView
    {
        // HACK: Pass in a function for instantiating the tile view because the hand
        // view controller doesn't currently have access to the prefabs for the tiles
        // (currently MatchController is setup with the asset config). Once the tile
        // config is factored out into a scriptable object (or something similar) we can
        // probably pass that in directly instead.
        public void CallTile(
            TileView discard,
            ICall call,
            Func<TileId, TileView> instantiateView)
        {
            switch (call)
            {
                case Call.Chii chii:
                {
                    // Remove two tiles from the player's hand.
                    RemoveFromHand(0);
                    RemoveFromHand(0);

                    // Instantiate the view objects for the two called tiles.
                    AddMeld(new List<TileView>()
                    {
                        discard,
                        instantiateView(chii.Element0),
                        instantiateView(chii.Element1),
                    });
                }
                break;

                case Call.Pon pon:
                {
                    // Remove two tiles from the player's hand.
                    RemoveFromHand(0);
                    RemoveFromHand(0);

                    // Instantiate the view objects for the two called tiles.
                    AddMeld(new List<TileView>()
                    {
                        discard,
                        instantiateView(pon.Element0),
                        instantiateView(pon.Element1),
                    });
                }
                break;

                case Call.Kan kan:
                {
                    // Remove three tiles from the player's hand.
                    RemoveFromHand(0);
                    RemoveFromHand(0);
                    RemoveFromHand(0);

                    // Instantiate the view objects for the three called tiles.
                    AddMeld(new List<TileView>()
                    {
                        discard,

                        // HACK: Instantiate tile views for the other 3 tiles in the meld using
                        // the discarded tile's ID. This is a bit gross, since it could
                        // potentially trip up any client-side state validation that we add later.
                        // Until then it's easier than looking up the correct IDs for the other
                        // three instances of the tile.
                        instantiateView(discard.Model.Id),
                        instantiateView(discard.Model.Id),
                        instantiateView(discard.Model.Id),
                    });
                }
                break;

                case Call.Ron ron:
                {
                    throw new NotImplementedException("Visualize ron call for remote hand");
                }
            }
        }

        public void FillWithDummyTiles(GameObject prefab)
        {
            AddTiles(Enumerable.Range(0, 13).Select(_ => Instantiate(prefab)));
        }

        public void DiscardTile(TileView tile)
        {
            Debug.Assert(
                HasCurrentDraw,
                "Discarding a tile from remote hand, but hand has no current draw!");

            RemoveCurrentDraw();
            AddDiscard(tile);
        }

        public void DrawDummyTile(GameObject prefab)
        {
            AddDrawTile(Instantiate(prefab));

            // TODO: Animate the draw action.
        }

        /// <summary>
        /// Remove the tile at the specified index from the hand view, destroying it
        /// automatically.
        /// </summary>
        ///
        /// <param name="index">The index of the tile view to remove.</param>
        ///
        /// <remarks>
        /// This function deliberately hides the method of the same name in the base
        /// class in order to avoid the case where we accidentally remove the tile from
        /// the hand without destroying it. For remote hands, the player's hand will
        /// always contain dummy tiles (because we don't know the actual tiles in the
        /// hand), so when we remove a tile from the hand we're always going to be
        /// replacing it with a new instance of the correct tile value.
        /// </remarks>
        private new void RemoveFromHand(int index)
        {
            Destroy(base.RemoveFromHand(index));
        }

        /// <summary>
        /// Remove the current draw tile from the hand view, destroying it automatically.
        /// </summary>
        ///
        /// <remarks>
        /// See the remarks on <see cref="RemoveFromHand"/> for notes on why this method
        /// hides the method of the same name in the base class.
        /// </remarks>
        private new void RemoveCurrentDraw()
        {
            Destroy(base.RemoveCurrentDraw());
        }
    }
}
