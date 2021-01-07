# HealthTrackerr

Small project to familirize myself with Actix.

Udp Actors broadcast 'health request' messages periodically (specified by delay) to each other and respond with 'health respond' messages.
When actor is requested with 'system health info request' it responds with message containing list of alive actors in the system.
