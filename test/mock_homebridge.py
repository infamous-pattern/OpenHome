#!/usr/bin/env python3
"""Small Homebridge Config UI API simulator for OpenDeck plugin testing."""

from __future__ import annotations

import argparse
import copy
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote

TOKEN = "mock-homebridge-token"

INITIAL_SERVICES = [
    {
        "aid": 2,
        "iid": 10,
        "uuid": "00000043-0000-1000-8000-0026BB765291",
        "type": "Lightbulb",
        "humanType": "Lightbulb",
        "serviceName": "Desk Lamp",
        "serviceType": "Lightbulb",
        "accessoryName": "Office Lamp",
        "accessoryInformation": {"Name": "Office Lamp", "Manufacturer": "Mock Lighting", "Model": "ML-100", "Serial Number": "LAMP-001", "Firmware Revision": "2.0"},
        "instance": {"name": "Mock Homebridge", "username": "00:11:22:33:44:55"},
        "uniqueId": "mock-light-service",
        "serviceCharacteristics": [
            {
                "uuid": "00000025-0000-1000-8000-0026BB765291",
                "type": "On",
                "description": "On",
                "value": False,
                "format": "bool",
                "canRead": True,
                "canWrite": True,
            },
            {
                "uuid": "00000008-0000-1000-8000-0026BB765291",
                "type": "Brightness",
                "description": "Brightness",
                "value": 40,
                "format": "int",
                "minValue": 0,
                "maxValue": 100,
                "minStep": 1,
                "canRead": True,
                "canWrite": True,
            },
            {
                "uuid": "00000013-0000-1000-8000-0026BB765291",
                "type": "Hue",
                "description": "Hue",
                "value": 180.0,
                "format": "float",
                "minValue": 0,
                "maxValue": 360,
                "minStep": 1,
                "canRead": True,
                "canWrite": True,
            },
        ],
    },
    {
        "aid": 3,
        "iid": 20,
        "uuid": "0000004A-0000-1000-8000-0026BB765291",
        "type": "Thermostat",
        "humanType": "Thermostat",
        "serviceName": "Office Thermostat",
        "serviceType": "Thermostat",
        "accessoryName": "Office Thermostat",
        "accessoryInformation": {"Name": "Office Thermostat", "Serial Number": "THERM-001"},
        "instance": {"name": "Mock Homebridge", "username": "00:11:22:33:44:55"},
        "uniqueId": "mock-thermostat-service",
        "serviceCharacteristics": [
            {
                "uuid": "temperature-current",
                "type": "CurrentTemperature",
                "description": "Current Temperature",
                "value": 21.5,
                "format": "float",
                "minValue": -40,
                "maxValue": 100,
                "minStep": 0.1,
                "canRead": True,
                "canWrite": False,
            },
            {
                "uuid": "temperature-target",
                "type": "TargetTemperature",
                "description": "Target Temperature",
                "value": 22.0,
                "format": "float",
                "minValue": 10,
                "maxValue": 30,
                "minStep": 0.5,
                "canRead": True,
                "canWrite": True,
            },
            {
                "uuid": "heating-state",
                "type": "TargetHeatingCoolingState",
                "description": "Target Mode",
                "value": 1,
                "format": "uint8",
                "minValue": 0,
                "maxValue": 3,
                "minStep": 1,
                "validValues": [0, 1, 2, 3],
                "canRead": True,
                "canWrite": True,
            },
        ],
    },
    {
        "aid": 4,
        "iid": 30,
        "uuid": "0000008C-0000-1000-8000-0026BB765291",
        "type": "WindowCovering",
        "humanType": "Window Covering",
        "serviceName": "Office Blind",
        "serviceType": "Window Covering",
        "accessoryName": "Office Blind",
        "accessoryInformation": {"Name": "Office Blind", "Serial Number": "BLIND-001"},
        "instance": {"name": "Mock Homebridge", "username": "00:11:22:33:44:55"},
        "uniqueId": "mock-blind-service",
        "serviceCharacteristics": [
            {
                "uuid": "target-position",
                "type": "TargetPosition",
                "description": "Target Position",
                "value": 0,
                "format": "uint8",
                "minValue": 0,
                "maxValue": 100,
                "minStep": 1,
                "canRead": True,
                "canWrite": True,
            }
        ],
    },
]

ROOMS = [
    {
        "name": "Office",
        "services": [
            {"uniqueId": "mock-light-service", "customName": "Desk Lamp"},
            {"uniqueId": "mock-thermostat-service", "customName": "Thermostat"},
            {"uniqueId": "mock-blind-service", "customName": "Blind"},
        ],
    }
]


class MockHomebridgeServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(
        self,
        address: tuple[str, int],
        require_auth: bool = False,
        username: str = "admin",
        password: str = "admin",
    ) -> None:
        super().__init__(address, Handler)
        self.require_auth = require_auth
        self.username = username
        self.password = password
        self.services = copy.deepcopy(INITIAL_SERVICES)
        self.rooms = copy.deepcopy(ROOMS)
        self.lock = threading.Lock()


class Handler(BaseHTTPRequestHandler):
    server: MockHomebridgeServer
    server_version = "MockHomebridge/2.0"

    def _json(self, status: int, payload: object) -> None:
        encoded = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def _body(self) -> dict:
        length = int(self.headers.get("Content-Length", "0"))
        if not length:
            return {}
        return json.loads(self.rfile.read(length).decode("utf-8"))

    def _authorised(self) -> bool:
        if self.headers.get("Authorization") == f"Bearer {TOKEN}":
            return True
        self._json(401, {"statusCode": 401, "message": "Missing or invalid token"})
        return False

    def _find_service(self, unique_id: str) -> dict | None:
        return next((service for service in self.server.services if service["uniqueId"] == unique_id), None)

    def do_POST(self) -> None:  # noqa: N802
        if self.path == "/api/auth/noauth":
            if self.server.require_auth:
                self._json(401, {"statusCode": 401, "message": "Authentication required"})
            else:
                self._json(201, {"access_token": TOKEN, "token_type": "Bearer", "expires_in": 3600})
            return

        if self.path == "/api/auth/login":
            body = self._body()
            if body.get("username") == self.server.username and body.get("password") == self.server.password:
                self._json(201, {"access_token": TOKEN, "token_type": "Bearer", "expires_in": 3600})
            else:
                self._json(401, {"statusCode": 401, "message": "Invalid username or password"})
            return

        self._json(404, {"message": "Not found"})

    def do_GET(self) -> None:  # noqa: N802
        if not self._authorised():
            return
        if self.path == "/api/accessories":
            with self.server.lock:
                self._json(200, self.server.services)
            return
        if self.path == "/api/accessories/layout":
            self._json(200, self.server.rooms)
            return
        prefix = "/api/accessories/"
        if self.path.startswith(prefix):
            unique_id = unquote(self.path[len(prefix):])
            with self.server.lock:
                service = self._find_service(unique_id)
                if service is None:
                    self._json(404, {"message": "Accessory not found"})
                else:
                    self._json(200, service)
            return
        self._json(404, {"message": "Not found"})

    def do_PUT(self) -> None:  # noqa: N802
        if not self._authorised():
            return
        prefix = "/api/accessories/"
        if not self.path.startswith(prefix):
            self._json(404, {"message": "Not found"})
            return

        unique_id = unquote(self.path[len(prefix):])
        body = self._body()
        characteristic_type = body.get("characteristicType")
        with self.server.lock:
            service = self._find_service(unique_id)
            if service is None:
                self._json(404, {"message": "Accessory not found"})
                return
            characteristic = next(
                (
                    item
                    for item in service.get("serviceCharacteristics", [])
                    if item.get("type") == characteristic_type
                ),
                None,
            )
            if characteristic is None:
                self._json(404, {"message": "Characteristic not found"})
                return
            if not characteristic.get("canWrite", False):
                self._json(400, {"message": "Characteristic is read-only"})
                return
            characteristic["value"] = body.get("value")
            self._json(200, service)

    def log_message(self, format: str, *args: object) -> None:
        print(f"{self.address_string()} - {format % args}")


def create_server(
    host: str = "127.0.0.1",
    port: int = 8581,
    require_auth: bool = False,
    username: str = "admin",
    password: str = "admin",
) -> MockHomebridgeServer:
    return MockHomebridgeServer((host, port), require_auth, username, password)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8581)
    parser.add_argument("--auth", action="store_true", help="Require username/password authentication")
    parser.add_argument("--username", default="admin")
    parser.add_argument("--password", default="admin")
    args = parser.parse_args()

    server = create_server(args.host, args.port, args.auth, args.username, args.password)
    mode = "username/password" if args.auth else "authentication disabled"
    print(f"Mock Homebridge listening at http://{args.host}:{args.port} ({mode})")
    server.serve_forever()


if __name__ == "__main__":
    main()
