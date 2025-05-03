FROM gcc:12 as builder
COPY /src/lkjscript.c /data/lkjscript.c
WORKDIR /data
RUN gcc -o lkjscript -static -O2 -march=native lkjscript.c

FROM scratch
WORKDIR /data
COPY --from=builder /data/lkjscript ./lkjscript
CMD ["./lkjscript"]