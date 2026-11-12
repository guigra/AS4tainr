**FREE
// OVERLAY: the subfields share the storage of PartNumber (the example of
// IBM Docs "OVERLAY(name{:start_pos | *NEXT})": Family 1-3, Sequence 4-9,
// Language 10). Start positions are explicit: as4tainr places *NEXT
// after the whole field instead (docs/native/fidelity.md).
dcl-ds part;
  PartNumber char(10);
  Family char(3) overlay(PartNumber);
  Sequence char(6) overlay(PartNumber : 4);
  Language char(1) overlay(PartNumber : 10);
end-ds;
PartNumber = 'ABC123456E';
if Family = 'ABC' and Sequence = '123456' and Language = 'E';
  dsply 'OVERLAY READ OK';
else;
  dsply 'OVERLAY READ FAIL';
endif;
Family = 'XYZ';
if PartNumber = 'XYZ123456E';
  dsply 'OVERLAY WRITE OK';
else;
  dsply 'OVERLAY WRITE FAIL';
endif;
*inlr = *on;
