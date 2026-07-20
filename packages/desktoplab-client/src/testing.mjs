export class InMemoryTransport {
  #requests = [];
  #responder;

  constructor({ testOnly, responder }) {
    if (testOnly !== true) throw new Error("InMemoryTransport is restricted to explicit test routing");
    if (typeof responder !== "function") throw new TypeError("test responder is required");
    this.#responder = responder;
  }

  async request(request) {
    this.#requests.push(structuredClone(request));
    return this.#responder(request);
  }

  requests() {
    return structuredClone(this.#requests);
  }
}
