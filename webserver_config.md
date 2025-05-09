
# Nginx
Proxy request buffering re-orders requests and consumes uploads whole until flushing to tapfer in an instant.  
The intermediate (but necessary) proxy buffer size is relatively arbitrary, 10 megabytes seems to work just fine.
```
location / {
    proxy_pass http://localhost:3003;
    proxy_request_buffering off;
    client_body_buffer_size 10M;
}
```