using System.Collections.Generic;
using EnumUtils;

namespace DotNetGame.Mahjong
{
    public static class TileSet
    {
        public static ITile[] GenerateTiles()
        {
            var tiles = new List<ITile>();

            // Add the simple tiles for each suit.
            foreach (var suit in EnumHelper.GetValues<Suit>())
            {
                for (var number = 1; number <= 9; number += 1)
                {
                    for (var count = 0; count < 4; count += 1)
                    {
                        tiles.Add(new SimpleTile(suit, number));
                    }
                }
            }

            // Add the honor tiles.
            foreach (var dragon in EnumHelper.GetValues<Dragon>())
            {
                for (var count = 0; count < 4; count += 1)
                {
                    tiles.Add(new HonorTile(dragon));
                }
            }

            foreach (var wind in EnumHelper.GetValues<Wind>())
            {
                for (var count = 0; count < 4; count += 1)
                {
                    tiles.Add(new HonorTile(wind));
                }
            }

            // Add the bonus tiles.
            foreach (var flower in EnumHelper.GetValues<Flower>())
            {
                tiles.Add(new BonusTile(flower));
            }

            foreach (var season in EnumHelper.GetValues<Season>())
            {
                tiles.Add(new BonusTile(season));
            }

            return tiles.ToArray();
        }
    }
}
