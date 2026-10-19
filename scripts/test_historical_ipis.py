import unittest
from import_historical_ipis import aggregate


def row(**kw):
    return dict(province='Nord-Kivu', visit_date='2023-03-01', pcode='a', vid='1',
                presence='1', interference='', armed_group1='FARDC',
                armed_group2='FARDC', armed_group3='', **kw)


class HistoricalImportTest(unittest.TestCase):
    def test_latest_visit_missing_values_and_actor_dedup(self):
        old = row()
        new = dict(old, vid='2', visit_date='2023-09-01', interference='0')
        d = aggregate([old, new])[0]
        self.assertEqual((d['sites'], d['visits']), (1, 2))
        self.assertEqual(d['interference'], dict(yes=0, no=1, unknown=0))
        self.assertEqual(d['actor_mentions'], [dict(name='FARDC', sites=1)])

    def test_years_do_not_carry_forward_and_recent_excluded(self):
        a = row()
        rows = [a, dict(a, visit_date='2024-01-01'), dict(a, visit_date='2021-01-01')]
        d = aggregate(rows)
        self.assertEqual([x['year'] for x in d], [2021, 2023])
        self.assertEqual(d[0]['interference'], dict(yes=0, no=0, unknown=1))

    def test_unknown_schema_fails(self):
        with self.assertRaises(ValueError):
            aggregate([dict(row(), interference='yes')])


if __name__ == '__main__':
    unittest.main()
