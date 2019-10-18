namespace DotNetGame.Mahjong
{
    public readonly struct HonorTile : ITile
    {
        private readonly Wind _wind;
        private readonly Dragon _dragon;

        public readonly HonorKind Kind;

        public Wind? Wind => Kind == HonorKind.Wind ? _wind : default;

        public Dragon? Dragon => Kind == HonorKind.Dragon ? _dragon : default;

        TileKind ITile.Kind => TileKind.Honor;

        public HonorTile(Dragon dragon)
        {
            Kind = HonorKind.Dragon;
            _dragon = dragon;
            _wind = default;
        }

        public HonorTile(Wind wind)
        {
            Kind = HonorKind.Wind;
            _wind = wind;
            _dragon = default;
        }
    }
}
