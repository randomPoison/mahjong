using System;
using Xunit;
using DotNetGame.Mahjong;

namespace Mahjong.Tests
{
    public class Mahjong_SimpleTileShould
    {
        [Fact]
        public void CreateTile_NumberIsValid_NotThrowException()
        {
            for (var number = 1; number < 10; number += 1)
            {
                var tile = new SimpleTile(Suit.Coins, number);
                Assert.True(tile.Number == number);
            }
        }

        [Fact]
        public void CreateTile_NumberIs0_Throw()
        {
            Assert.Throws<ArgumentException>(() => new SimpleTile(Suit.Coins, 0));
        }

        [Fact]
        public void CreateTile_NumberIs10_Throw()
        {
            Assert.Throws<ArgumentException>(() => new SimpleTile(Suit.Coins, 10));
        }
    }
}
