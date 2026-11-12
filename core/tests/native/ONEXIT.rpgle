**FREE
// ON-EXIT: a RETURN in ON-EXIT replaces the value after a normal end
// (IBM Docs "ON-EXIT (On Exit)": 5 returned by the body, 6 by ON-EXIT).
ctl-opt dftactgrp(*no);
dcl-s r int(10);
r = f();
if r = 6;
  dsply 'ONEXIT OK';
else;
  dsply 'ONEXIT FAIL';
endif;
*inlr = *on;

dcl-proc f;
  dcl-pi *n int(10);
  end-pi;
  return 5;
on-exit;
  return 6;
end-proc;
