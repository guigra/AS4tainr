**FREE
// Native unit NDSPATH (S2.209): a DS array subfield of a DS array element
// as a CONST LIKEDS DIM argument, the program captured on PUB400 as
// GN209DP with DSPLY for its output.
// Source: golden capture tools/golden/native209 n209_dspath (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-ds m_t qualified template;
  parmNo int(10);
  val varchar(20);
end-ds;
dcl-ds gv qualified dim(3) inz;
  nm int(10);
  m likeds(m_t) dim(8);
end-ds;
dcl-s s int(10) inz(3);
dcl-s line varchar(200);

gv(s).nm = 3;
gv(s).m(1).parmNo = 1;
gv(s).m(2).parmNo = 2;
gv(s).m(3).parmNo = 3;
gv(s).m(3).val = 'three';
gv(1).m(1).parmNo = 9;
line = show(gv(s).m : gv(s).nm) + '|' + show(gv(1).m : 1);
dsply (line);
*inlr = *on;

dcl-proc show;
  dcl-pi *n varchar(50);
    m likeds(m_t) dim(8) const;
    n int(10) const;
  end-pi;
  dcl-s i int(10);
  dcl-s o varchar(50);
  for i = 1 to n;
    o += %char(m(i).parmNo) + ' ';
  endfor;
  return o + m(3).val;
end-proc;
