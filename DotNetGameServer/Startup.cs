using System;
using System.Net.WebSockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using DotNetGame.Mahjong;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Newtonsoft.Json;

namespace DotNetGame
{
    public class Startup
    {
        // TODO: Find a proper, non-static place to hold the tiles and other game state.
        private readonly static ITile[] Tiles = TileSet.GenerateTiles();

        public Startup(IConfiguration configuration)
        {
            Configuration = configuration;
        }

        public IConfiguration Configuration { get; }

        // This method gets called by the runtime. Use this method to add services to the container.
        public void ConfigureServices(IServiceCollection services)
        {
            services.Configure<CookiePolicyOptions>(options =>
            {
                // This lambda determines whether user consent for non-essential cookies is needed for a given request.
                options.CheckConsentNeeded = context => true;
                options.MinimumSameSitePolicy = SameSiteMode.None;
            });

            services.AddMvc().SetCompatibilityVersion(CompatibilityVersion.Version_2_2);
        }

        // This method gets called by the runtime. Use this method to configure the HTTP request pipeline.
        public void Configure(IApplicationBuilder app, IHostingEnvironment env)
        {
            if (env.IsDevelopment())
            {
                app.UseDeveloperExceptionPage();
            }
            else
            {
                app.UseExceptionHandler("/Error");
            }

            app.UseStaticFiles();
            app.UseCookiePolicy();
            app.UseWebSockets();
            app.UseMvc();

            // Setup websocket endpoint.
            app.Use(async (context, next) =>
            {
                if (context.Request.Path == "/ws")
                {
                    if (context.WebSockets.IsWebSocketRequest)
                    {
                        var webSocket = await context.WebSockets.AcceptWebSocketAsync();
                        await Echo(context, webSocket);
                    }
                    else
                    {
                        context.Response.StatusCode = 400;
                    }
                }
                else
                {
                    await next();
                }
            });
        }

        private async Task Echo(HttpContext context, WebSocket socket, CancellationToken cancellation = default)
        {
            var buffer = new byte[1024 * 4];

            // When a client first connects, send them the current set of tiles.
            var tilesMessage = JsonConvert.SerializeObject(Tiles);
            await socket.SendAsync(
                Encoding.UTF8.GetBytes(tilesMessage),
                WebSocketMessageType.Text,
                true,
                cancellation);

            var result = await socket.ReceiveAsync(
                buffer,
                cancellation);

            while (!result.CloseStatus.HasValue)
            {
                await socket.SendAsync(
                    new ArraySegment<byte>(buffer, 0, result.Count),
                    result.MessageType,
                    result.EndOfMessage,
                    cancellation);

                result = await socket.ReceiveAsync(
                    new ArraySegment<byte>(buffer),
                    cancellation);
            };

            await socket.CloseAsync(
                result.CloseStatus.Value,
                result.CloseStatusDescription,
                cancellation);
        }
    }
}
