**FREE
// Native unit NOMITLC (S2.195): *OMIT for a procedure of the module, the
// program captured on PUB400 as GN195OM with DSPLY for its output.
// Source: golden capture tools/golden/native195 n195_omit (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-s line varchar(200);

line = f(1) + ' ' + f(1 : *omit) + ' ' + f(1 : 'x');
dsply (line);
*inlr = *on;

dcl-proc f;
  dcl-pi *n varchar(20);
    a int(10) const;
    b char(10) const options(*nopass : *omit);
  end-pi;
  dcl-s r varchar(20);
  r = %char(%parms());
  if %parms() >= 2;
    if %addr(b) = *null;
      r += 'N';
    else;
      r += %trim(b);
    endif;
  endif;
  return r;
end-proc;
