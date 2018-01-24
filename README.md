# HTTP ARF
## The HTTP watchdog

`harf` barks when your services are down, making it easy to restart them either via `systemd` integration or by arbitrary shell commands.
Unix only.

Sample config:

```
{
    "sites": [{
        "name": "Example Website",
        "url": "http://example.com/",
        "unit": "mainsite_fake.service"
    },
    {
        "name": "Example Website 404",
        "url": "http://example.com/nonexist/",
        "unit": "mainsite_fake.service"
    },
    {
        "name": "Test Site",
        "url": "http://localhost:8080",
        "cmd": "echo 'Test service is down!' > test.txt"
    }]
}
```

Sample output:

```
harf - HTTP Watchdog v.0.1.0

[NOT OK] Test Site
         |connection refused
         |Running command 'echo 'Test service is down!' > test.txt'
         +------------

[OK]     Example Website
[NOT OK] Example Website 404
         |404 Not Found
         |Restarting unit 'mainsite_fake.service'
         +------------
```
