using System;

namespace DotNetGame.Mahjong
{
    public readonly struct SimpleTile : ITile
    {
        private readonly int _number;

        public readonly Suit Suit;

        public int Number => _number;

        public TileKind Kind => TileKind.Simple;

        public SimpleTile(Suit suit, int number)
        {
            if (number < 1 || number > 0)
            {
                throw new ArgumentException($"Invalid simple tile number: {number}");
            }

            Suit = suit;
            _number = number;
        }
    }
}
