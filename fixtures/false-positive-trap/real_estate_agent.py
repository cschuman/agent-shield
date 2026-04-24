"""A real-estate agent class. Has nothing to do with AI agents."""


class RealEstateAgent:
    def __init__(self, name: str, license_id: str):
        self.name = name
        self.license_id = license_id

    def list_property(self, address: str) -> dict:
        return {"agent": self.name, "address": address}
