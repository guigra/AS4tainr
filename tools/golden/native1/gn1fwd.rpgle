**FREE
// A CONST parameter passed on to a procedure that changes its
// parameter: the compiler's answer.
ctl-opt dftactgrp(*no);
dcl-s r int(10) inz(5);
a(r);
*inlr = *on;
dcl-proc a;
  dcl-pi *n;
    p int(10) const;
  end-pi;
  b(p);
end-proc;
dcl-proc b;
  dcl-pi *n;
    q int(10);
  end-pi;
  q = 9;
end-proc;
