To test CORS, setup two duckrg like this (suppose to be in project's base dir):

```bash
duckrg --mem-db test::tests/cors/test.yaml &
duckrg --serve-dir tests/cors/ --port 12322 --index-file index.html &
```

Then visit `http://localhost:12322` with a browser; in the network debugger you should find that the OPTIONS call (preflight) and the POST call are successful.