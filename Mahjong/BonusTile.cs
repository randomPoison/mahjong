namespace DotNetGame.Mahjong
{
    public readonly struct BonusTile : ITile
    {
        private readonly Flower _flower;
        private readonly Season _season;

        public readonly BonusKind Kind;

        public Flower? Flower => Kind == BonusKind.Flower ? _flower : default;

        public Season? Season => Kind == BonusKind.Season ? _season : default;

        TileKind ITile.Kind => TileKind.Bonus;

        public BonusTile(Flower flower)
        {
            Kind = BonusKind.Flower;
            _flower = flower;
            _season = default;
        }

        public BonusTile(Season season)
        {
            Kind = BonusKind.Season;
            _season = season;
            _flower = default;
        }
    }
}
