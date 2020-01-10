using System.Runtime.InteropServices;

namespace Synapse.Mahjong
{
    public class Mahjong
    {
        [DllImport("__Internal")]
        public static extern string GenerateTileset();
    }
}
