using DotNetGame.Mahjong;
using Newtonsoft.Json;
using Xunit;

namespace Mahjong.Tests
{
    public class Mahjong_TileSerializationShould
    {
        [Fact]
        public void SimpleTile_SerializeJson()
        {
            var tile = new SimpleTile(Suit.Coins, 1);
            var json = JsonConvert.ToString(tile);
        }

        [Fact]
        public void SimpleTile_AsITile_SerializeJson()
        {
            ITile tile = new SimpleTile(Suit.Coins, 1);
            var json = JsonConvert.ToString(tile);
        }
    }
}
