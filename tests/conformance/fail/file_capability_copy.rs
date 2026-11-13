fn consume(_file: File) {}
fn probe(file: File) {
    consume(file);
    consume(file);
}
