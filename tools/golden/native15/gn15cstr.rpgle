**FREE
ctl-opt dftactgrp(*no);
dcl-s r int(10);
r = f(5);
*inlr = *on;
dcl-proc f;
  dcl-pi *n int(10);
    p int(10) const;
  end-pi;
  return p + 1;
end-proc;
