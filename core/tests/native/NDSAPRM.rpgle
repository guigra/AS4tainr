**FREE
// Native unit NDSAPRM (S2.169): a data structure array for a CONST LIKEDS
// DIM parameter, the program captured on PUB400 as GN169DA with DSPLY for
// its output.
// Source: golden capture tools/golden/native169 n169_dsaparm (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-ds m_t qualified template;
  parmNo int(10);
  val varchar(20);
end-ds;
dcl-ds arr likeds(m_t) dim(4) inz;
dcl-s line varchar(120);

arr(1).parmNo = 1;
arr(2).parmNo = 2;
arr(2).val = 'two';
arr(3).parmNo = 3;
line = show(arr : 3);
line += '|' + inner();
dsply (line);
*inlr = *on;

dcl-proc show;
  dcl-pi *n varchar(60);
    m likeds(m_t) dim(4) const;
    n int(10) const;
  end-pi;
  dcl-s i int(10);
  dcl-s o varchar(60);
  for i = 1 to n;
    o += %char(m(i).parmNo) + m(i).val + ' ';
  endfor;
  return o;
end-proc;

dcl-proc inner;
  dcl-pi *n varchar(60);
  end-pi;
  dcl-ds loc likeds(m_t) dim(4) inz;
  loc(1).parmNo = 9;
  loc(1).val = 'x';
  loc(4).parmNo = 4;
  loc(4).val = 'four';
  return show(loc : 4);
end-proc;
