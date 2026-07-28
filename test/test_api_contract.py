from __future__ import annotations

import json
import threading
import unittest
import urllib.error
import urllib.request

from test.mock_homebridge import TOKEN, create_server


class ApiContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.server = create_server(port=0)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        host, port = self.server.server_address
        self.base = f"http://{host}:{port}"

    def tearDown(self) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=2)

    def request(self, path: str, method: str = "GET", body: object | None = None, token: str | None = TOKEN):
        data = None if body is None else json.dumps(body).encode()
        headers = {"Content-Type": "application/json"}
        if token:
            headers["Authorization"] = f"Bearer {token}"
        request = urllib.request.Request(self.base + path, data=data, method=method, headers=headers)
        with urllib.request.urlopen(request) as response:
            return response.status, json.loads(response.read())

    def test_noauth_and_accessory_discovery(self) -> None:
        status, token = self.request("/api/auth/noauth", method="POST", token=None)
        self.assertEqual(status, 201)
        self.assertEqual(token["access_token"], TOKEN)
        status, services = self.request("/api/accessories")
        self.assertEqual(status, 200)
        self.assertGreaterEqual(len(services), 3)
        self.assertTrue(any(item["uniqueId"] == "mock-light-service" for item in services))

    def test_layout_and_single_service(self) -> None:
        _, rooms = self.request("/api/accessories/layout")
        self.assertEqual(rooms[0]["name"], "Office")
        _, service = self.request("/api/accessories/mock-light-service")
        self.assertEqual(service["serviceName"], "Desk Lamp")

    def test_writes_characteristic_and_returns_updated_service(self) -> None:
        _, service = self.request(
            "/api/accessories/mock-light-service",
            method="PUT",
            body={"characteristicType": "On", "value": True},
        )
        on = next(item for item in service["serviceCharacteristics"] if item["type"] == "On")
        self.assertTrue(on["value"])
        _, refreshed = self.request("/api/accessories/mock-light-service")
        on = next(item for item in refreshed["serviceCharacteristics"] if item["type"] == "On")
        self.assertTrue(on["value"])


    def test_writes_brightness_and_returns_updated_service(self) -> None:
        _, service = self.request(
            "/api/accessories/mock-light-service",
            method="PUT",
            body={"characteristicType": "Brightness", "value": 75},
        )
        brightness = next(item for item in service["serviceCharacteristics"] if item["type"] == "Brightness")
        self.assertEqual(brightness["value"], 75)
        _, refreshed = self.request("/api/accessories/mock-light-service")
        brightness = next(item for item in refreshed["serviceCharacteristics"] if item["type"] == "Brightness")
        self.assertEqual(brightness["value"], 75)

    def test_rejects_read_only_characteristic(self) -> None:
        with self.assertRaises(urllib.error.HTTPError) as context:
            self.request(
                "/api/accessories/mock-thermostat-service",
                method="PUT",
                body={"characteristicType": "CurrentTemperature", "value": 25},
            )
        self.assertEqual(context.exception.code, 400)


class AuthenticatedApiTests(unittest.TestCase):
    def setUp(self) -> None:
        self.server = create_server(port=0, require_auth=True, username="admin", password="secret")
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        host, port = self.server.server_address
        self.base = f"http://{host}:{port}"

    def tearDown(self) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=2)

    def post(self, path: str, body: object):
        request = urllib.request.Request(
            self.base + path,
            data=json.dumps(body).encode(),
            method="POST",
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(request) as response:
            return response.status, json.loads(response.read())

    def test_login(self) -> None:
        with self.assertRaises(urllib.error.HTTPError) as context:
            self.post("/api/auth/login", {"username": "admin", "password": "wrong"})
        self.assertEqual(context.exception.code, 401)
        status, token = self.post("/api/auth/login", {"username": "admin", "password": "secret"})
        self.assertEqual(status, 201)
        self.assertEqual(token["access_token"], TOKEN)


if __name__ == "__main__":
    unittest.main()
