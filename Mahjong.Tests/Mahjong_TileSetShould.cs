using System;
using Xunit;
using DotNetGame.Mahjong;

namespace Mahjong.Tests
{
    public class Mahjong_TileSetShould
    {
        private readonly ITile[] _tiles;

        public Mahjong_TileSetShould()
        {
            _tiles = TileSet.GenerateTiles();
        }

        [Fact]
        public void GeneratesTileSet()
        {
            Assert.Equal(144, _tiles.Length);
        }
    }
}
