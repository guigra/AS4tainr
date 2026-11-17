**FREE
// Native unit NLOCADR (S2.65): %ADDR of a procedure's local fields. A
// based field over a local, pointer arithmetic along a local array, and
// the address of a parameter passed by reference (the caller's storage).
// The program captured on PUB400, with DSPLY for its output.
// Source: golden capture tools/golden/native65 n65_adr (PUB400, IBM i
// 7.5, 2026-11-17).
ctl-opt dftactgrp(*no);
dcl-s outer char(6) inz('ABCDEF');
dsply ('VIA ' + via());
dsply ('WALK ' + %char(walk()));
poke(outer);
dsply ('POKE ' + outer);
*inlr = *on;

dcl-proc via;
  dcl-pi *n char(5) end-pi;
  dcl-s word char(5) inz('hello');
  dcl-s p pointer;
  dcl-s seen char(5) based(p);
  p = %addr(word);
  %subst(seen : 1 : 1) = 'J';
  return seen;
end-proc;

dcl-proc walk;
  dcl-pi *n int(10) end-pi;
  dcl-s nums int(10) dim(4);
  dcl-s p pointer;
  dcl-s n int(10) based(p);
  dcl-s i int(10);
  dcl-s t int(10);
  for i = 1 to 4;
    nums(i) = i * i;
  endfor;
  p = %addr(nums(2));
  t = n;
  p += %size(n);
  t += n * 10;
  return t;
end-proc;

dcl-proc poke;
  dcl-pi *n;
    target char(6);
  end-pi;
  dcl-s p pointer;
  dcl-s c char(1) based(p);
  p = %addr(target);
  p += 2;
  c = 'z';
end-proc;
