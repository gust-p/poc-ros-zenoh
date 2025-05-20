@0xf0490882a167f562;

struct Echo {
  msg @0 :Text;
}

interface EchoService {
  doEcho @0 (data: Echo) -> ();
}
