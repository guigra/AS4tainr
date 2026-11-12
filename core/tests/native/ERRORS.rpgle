**FREE
// Status 00102 (divide by zero) and 00103 (a result too large for its
// field) taken by ON-ERROR; control goes on after ENDMON.
dcl-s zero packed(5:0) inz(0);
dcl-s q packed(5:0);
dcl-s small packed(3:0) inz(999);
monitor;
  q = 10 / zero;
  dsply 'DIV0 FAIL';
on-error 00102;
  dsply 'DIV0 OK';
endmon;
monitor;
  small = small + 1;
  dsply 'OVERFLOW FAIL';
on-error 00103;
  dsply 'OVERFLOW OK';
endmon;
monitor;
  q = 10 / zero;
on-error *program;
  dsply 'PROGRAM OK';
endmon;
dsply 'AFTER ENDMON';
*inlr = *on;
