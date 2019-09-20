using System;
using System.Collections.Generic;
using System.Linq;

namespace DotNetGame.Mahjong
{
    public static class TileSet
    {
        public static ITile[] GenerateTiles()
        {
            var tiles = new List<ITile>();

            // Add the simple tiles for each suit.
            foreach (var suit in Enum.GetValues(typeof(Suit)).Cast<Suit>())
            {
                for (var number = 1; number <= 9; number += 1)
                {
                    tiles.Add(new SimpleTile(suit, number));
                }
            }

            // Add the honor tiles.
            foreach (var dragon in Enum.GetValues(typeof(Dragon)).Cast<Dragon>())
            {
                tiles.Add(new HonorTile(dragon));
            }

            foreach (var wind in Enum.GetValues(typeof(Wind)).Cast<Wind>())
            {
                tiles.Add(new HonorTile(wind));
            }

            // Add the bonus tiles.
            foreach (var flower in Enum.GetValues(typeof(Flower)).Cast<Flower>())
            {
                tiles.Add(new BonusTile(flower));
            }

            foreach (var season in Enum.GetValues(typeof(Season)).Cast<Season>())
            {
                tiles.Add(new BonusTile(season));
            }

            return tiles.ToArray();
        }
    }
}
