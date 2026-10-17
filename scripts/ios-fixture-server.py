#!/usr/bin/env python3
"""Loopback fixtures for native simulator tests; application HTTP stays in Rust."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse, parse_qs
import json
import os
import sys
import threading

LUKE = json.loads((Path(__file__).resolve().parents[1] / 'crates/swapi-core/tests/fixtures/luke.json').read_text())
DROID = dict(LUKE, name='C-3PO', height='167', url='https://swapi.dev/api/people/2/')
LEIA = dict(LUKE, name='Leia Organa', height='150', url='https://swapi.dev/api/people/5/')
RELEASE = threading.Event()


class Handler(BaseHTTPRequestHandler):
    scenario = 'people'
    detail_requests = 0
    page_requests = 0

    def respond(self, status, value=None):
        body = b'' if value is None else json.dumps(value).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        try:
            self.wfile.write(body)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_GET(self):
        url = urlparse(self.path)
        params = parse_qs(url.query)
        if url.path == '/reset':
            Handler.scenario = params.get('scenario', ['people'])[0]
            Handler.detail_requests = Handler.page_requests = 0
            RELEASE.clear()
            return self.respond(200)
        if url.path == '/release':
            RELEASE.set()
            return self.respond(200)
        if url.path == '/api/people/':
            query = params.get('search', [''])[0].casefold()
            page = int(params.get('page', ['1'])[0])
            records = [p for p in [LUKE, DROID, LEIA] if query in p['name'].casefold()]
            if Handler.scenario == 'performance':
                records = [dict(LUKE, name=f'Explorer {i}', url=f'https://swapi.dev/api/people/{i}/') for i in range(1, 41)]
                return self.respond(200, dict(count=40, next=None, results=records))
            if not query and page == 2:
                Handler.page_requests += 1
                if Handler.page_requests == 1:
                    return self.respond(503)
            next_url = f'http://127.0.0.1:{self.server.server_port}/api/people/?page=2' if not query and page == 1 else None
            results = records if query else (records[:2] if page == 1 else records[2:])
            return self.respond(200, dict(count=len(records), next=next_url, results=results))
        records = {'/api/people/1/': LUKE, '/api/people/2/': DROID, '/api/people/5/': LEIA}
        if url.path in records:
            Handler.detail_requests += 1
            if Handler.scenario == 'detail' and Handler.detail_requests == 1:
                RELEASE.wait(30)
                return self.respond(503)
            return self.respond(200, records[url.path])
        self.respond(404)


server = ThreadingHTTPServer(('127.0.0.1', 18765), Handler)
Path(sys.argv[1]).write_text(str(os.getpid()))
server.serve_forever()
