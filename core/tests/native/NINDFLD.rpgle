**FREE
// Native unit NINDFLD (S2.100): indicator fields, the program captured
// on PUB400 as GN101IND with DSPLY for its output.
// Source: golden capture tools/golden/native101 n101_indfld (PUB400, IBM i
// 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s flag ind;
dcl-s spare ind inz(*on);
dcl-ds st qualified;
  done ind;
  n int(5);
end-ds;
dcl-s a int(5) inz(3);
dcl-s b int(5) inz(5);
dcl-s k int(5);
dcl-s line varchar(80);
flag = *on;
line = yn(flag);
flag = a > b;
line += yn(flag);
flag = spare;
line += yn(flag);
spare = *off;
if not spare;
  line += 'n';
endif;
st.done = a < b;
line += yn(st.done);
k = 0;
dow not st.done = *off and k < 3;
  k += 1;
  if k = 2;
    st.done = *off;
  endif;
enddo;
line += %char(k);
st.done = *off;
k = 0;
dou st.done;
  k += 1;
  st.done = k >= 4;
enddo;
line += %char(k) + ' ' + local(7);
dsply (line);
*inlr = *on;

dcl-proc yn;
  dcl-pi *n char(1);
    f ind value;
  end-pi;
  if f;
    return 'Y';
  endif;
  return 'N';
end-proc;

dcl-proc local;
  dcl-pi *n varchar(10);
    n int(5) value;
  end-pi;
  dcl-s big ind;
  big = n > 5;
  if big;
    return 'big';
  endif;
  return 'small';
end-proc;
