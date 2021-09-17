#[derive(Debug)]
pub enum DiffResult<T> {
    Remove(T),
    Update(T, T),
    Keep(T, T),
    Insert(T),
}

pub fn update_diff<'a, T: PartialEq, F>(
    old: &'a [T],
    new: &'a [T],
    can_update: F,
) -> Vec<DiffResult<&'a T>>
where
    F: Fn(&T, &T) -> bool,
{
    let mut diff = diff::slice(old, new);
    let mut res = Vec::<DiffResult<&'a T>>::new();
    let mut i = 0;
    // Convert <Remove, [Remove...]?, Add> sequences to <Update, [Remove...]?`>
    // for items where can_update returns true.
    while i < diff.len() {
        let cur = &diff[i];
        match cur {
            diff::Result::Left(remove) => {
                let mut next_add = i + 1;
                let mut next_add_value: Option<&T> = None;
                while next_add < diff.len() {
                    match &diff[next_add] {
                        diff::Result::Left(_) => {
                            next_add += 1;
                        }
                        diff::Result::Both(_, _) => {
                            next_add = diff.len();
                        }
                        diff::Result::Right(add) => {
                            next_add_value.replace(add);
                            break;
                        }
                    }
                }
                if next_add < diff.len() && can_update(remove, next_add_value.as_ref().unwrap()) {
                    res.push(DiffResult::Update(remove, next_add_value.as_ref().unwrap()));
                    diff.remove(next_add);
                } else {
                    res.push(DiffResult::Remove(remove));
                }
            }
            diff::Result::Both(same1, same2) => res.push(DiffResult::Keep(same1, same2)),
            diff::Result::Right(added) => res.push(DiffResult::Insert(added)),
        }
        i += 1;
    }
    res
}
