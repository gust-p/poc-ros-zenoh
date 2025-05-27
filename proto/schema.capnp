@0xf0490882a167f562;

struct Hello {
  msg @0 :Text;
}

interface HelloService {
  doHello @0 (data: Hello) -> ();
}

struct Vector3 {
  x @0 :Float32;
  y @1 :Float32;
  z @2 :Float32;
}

struct Twist  {
  linear @0 :Vector3;
  angular @1 :Vector3;
}

interface TwistService {
  doTwist @0 (data: Twist) -> ();
}

interface Bootstrap {
  getHelloService @0 () -> (service: HelloService);
  getTwistService @1 () -> (service: TwistService);
}
