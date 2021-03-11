from starlette.applications import Starlette
from starlette.responses import PlainTextResponse
from starlette.routing import Route


async def index(request):
    return PlainTextResponse("Hello, World!")


app = Starlette(routes=[
    Route("/", index),
])
