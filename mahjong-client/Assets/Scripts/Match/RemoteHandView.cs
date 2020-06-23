using System;
using System.Linq;
using UniRx.Async;
using UnityEngine;

namespace Synapse.Mahjong.Match
{
    public sealed class RemoteHandView : PlayerHandView
    {
        public override void CallTile(TileView discard, ICall call)
        {
            throw new NotImplementedException("Implement calling for remote hand");
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

        public async UniTask DrawDummyTile()
        {
            throw new NotImplementedException();

            // TODO: Animate the draw action. This delay is just here as a placeholder
            // to ensure the code handles the delay that will eventually be here once we
            // implement an animation.
            await UniTask.Delay(500);
        }
    }
}
