using System;
using System.IO;
using DotNetGame.Mahjong;
using Newtonsoft.Json;
using Xunit;
using Xunit.Abstractions;

namespace Mahjong.Tests
{
    public class Mahjong_TileSerializationShould
    {
        private readonly ITestOutputHelper _output;

        public Mahjong_TileSerializationShould(ITestOutputHelper output)
        {
            _output = output;
        }

        [Fact]
        public void SimpleTile_SerializeJson()
        {
            var tile = new SimpleTile(Suit.Coins, 1);

            var json = JsonConvert.SerializeObject(tile);
            _output.WriteLine(json);
        }

        [Fact]
        public void SimpleTile_AsITile_SerializeJson()
        {
            ITile tile = new SimpleTile(Suit.Coins, 1);
            var json = JsonConvert.SerializeObject(tile);
        }

        [Fact]
        public void SimpleTile_AsITile_DeserializeJson()
        {
            ITile tile = new SimpleTile(Suit.Coins, 1);
            var json = JsonConvert.SerializeObject(tile);

            ITile result = JsonConvert.DeserializeObject<ITile>(json);
            SimpleTile resultTile = (SimpleTile)result;
            Assert.Equal(Suit.Coins, resultTile.Suit);
            Assert.Equal(1, resultTile.Number);
        }
    }
}
