using System;
using Newtonsoft.Json;

namespace DotNetGame.Mahjong
{
    public readonly struct SimpleTile : ITile
    {
        [JsonProperty("number")]
        private readonly int _number;

        [JsonProperty("suit")]
        public readonly Suit Suit;

        public int Number => _number;

        public TileKind Kind => TileKind.Simple;

        [JsonConstructor]
        public SimpleTile(Suit suit, int number)
        {
            if (number < 1 || number > 9)
            {
                throw new ArgumentException($"Invalid simple tile number: {number}");
            }

            Suit = suit;
            _number = number;
        }
    }
}
